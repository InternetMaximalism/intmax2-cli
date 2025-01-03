use balance_logic::{
    receive_deposit, receive_transfer, update_no_send, update_send_by_receiver,
    update_send_by_sender,
};
use error::SyncError;
use intmax2_interfaces::{
    api::{
        balance_prover::interface::BalanceProverClientInterface,
        block_builder::interface::BlockBuilderClientInterface,
        store_vault_server::interface::StoreVaultClientInterface,
        validity_prover::interface::ValidityProverClientInterface,
        withdrawal_server::interface::WithdrawalServerClientInterface,
    },
    data::{
        common_tx_data::CommonTxData, deposit_data::DepositData, meta_data::MetaData,
        transfer_data::TransferData, tx_data::TxData, user_data::UserData,
    },
};
use intmax2_zkp::{
    circuits::balance::{balance_pis::BalancePublicInputs, send::spent_circuit::SpentPublicInputs},
    common::{
        signature::key_set::KeySet,
        witness::{transfer_witness::TransferWitness, withdrawal_witness::WithdrawalWitness},
    },
    ethereum_types::u256::U256,
};

use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use utils::generate_salt;

pub mod balance_logic;
pub mod error;
pub mod utils;

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

use crate::client::strategy::strategy::ReceiveAction;

use super::{
    client::Client,
    strategy::strategy::{determine_sequence, determine_withdrawals, Action, PendingInfo},
};

impl<BB, S, V, B, W> Client<BB, S, V, B, W>
where
    BB: BlockBuilderClientInterface,
    S: StoreVaultClientInterface,
    V: ValidityProverClientInterface,
    B: BalanceProverClientInterface,
    W: WithdrawalServerClientInterface,
{
    /// Get the latest user data from the data store server
    pub async fn get_user_data(&self, key: KeySet) -> Result<UserData, SyncError> {
        let user_data = self
            .store_vault_server
            .get_user_data(key.pubkey)
            .await?
            .map(|encrypted| UserData::decrypt(&encrypted, key))
            .transpose()
            .map_err(|e| SyncError::DecryptionError(format!("failed to decrypt user data: {}", e)))?
            .unwrap_or(UserData::new(key.pubkey));
        Ok(user_data)
    }

    /// Sync the client's balance proof with the latest block
    pub async fn sync(&self, key: KeySet) -> Result<PendingInfo, SyncError> {
        let (sequence, pending) = determine_sequence(
            &self.store_vault_server,
            &self.validity_prover,
            &self.liquidity_contract,
            key,
            self.config.deposit_timeout,
            self.config.tx_timeout,
        )
        .await?;
        for action in sequence {
            match action {
                Action::Receive {
                    receives,
                    new_deposit_lpt,
                    new_transfer_lpt,
                } => {
                    if !receives.is_empty() {
                        let largest_block_number = receives
                            .iter()
                            .map(|r| r.meta().block_number.unwrap())
                            .max()
                            .unwrap(); // safe to unwrap
                        self.update_no_send(key, largest_block_number).await?;
                        for receive in receives {
                            match receive {
                                ReceiveAction::Deposit(meta, data) => {
                                    self.sync_deposit(key, &meta, &data).await?;
                                }
                                ReceiveAction::Transfer(meta, data) => {
                                    self.sync_transfer(key, &meta, &data).await?;
                                }
                            }
                        }
                    }
                    self.update_deposit_lpt(key, new_deposit_lpt).await?;
                    self.update_transfer_lpt(key, new_transfer_lpt).await?;
                }
                Action::Tx(meta, tx_data) => {
                    self.sync_tx(key, &meta, &tx_data).await?;
                }
                Action::PendingReceives(meta, _tx_data) => {
                    return Err(SyncError::PendingReceivesError(format!(
                        "pending receives to proceed tx: {:?}",
                        meta.uuid
                    )));
                }
                Action::PendingTx(meta, _tx_data) => {
                    return Err(SyncError::PendingTxError(format!(
                        "pending tx: {:?}",
                        meta.uuid
                    )));
                }
            }
        }
        Ok(pending)
    }

    /// Sync the client's withdrawals and relays to the withdrawal server
    pub async fn sync_withdrawals(&self, key: KeySet) -> Result<(), SyncError> {
        let (withdrawals, new_withdrawal_lpt) = determine_withdrawals(
            &self.store_vault_server,
            &self.validity_prover,
            key,
            self.config.tx_timeout,
        )
        .await?;
        for (meta, data) in &withdrawals {
            self.sync_withdrawal(key, meta, data).await?;
        }
        self.update_withdrawal_lpt(key, new_withdrawal_lpt).await?;
        Ok(())
    }

    // sync deposit without updating the timestamp
    async fn sync_deposit(
        &self,
        key: KeySet,
        meta: &MetaData,
        deposit_data: &DepositData,
    ) -> Result<(), SyncError> {
        log::info!("sync_deposit: {:?}", meta);
        if meta.block_number.is_none() {
            return Err(SyncError::BlockNumberIsNotSetForMetaData);
        }
        let mut user_data = self.get_user_data(key).await?;

        // user's balance proof before applying the tx
        let prev_balance_proof = self
            .store_vault_server
            .get_balance_proof(
                key.pubkey,
                user_data.block_number,
                user_data.private_commitment(),
            )
            .await?;

        let new_salt = generate_salt();
        let new_balance_proof = receive_deposit(
            &self.validity_prover,
            &self.balance_prover,
            key,
            &mut user_data.full_private_state,
            new_salt,
            &prev_balance_proof,
            deposit_data,
        )
        .await?;

        let new_block_number = meta.block_number.unwrap().max(user_data.block_number);
        let new_balance_pis = BalancePublicInputs::from_pis(&new_balance_proof.public_inputs);
        if new_balance_pis.public_state.block_number != new_block_number {
            return Err(SyncError::BalanceProofBlockNumberMismatch {
                balance_proof_block_number: new_balance_pis.public_state.block_number,
                block_number: new_block_number,
            });
        }

        // update user data
        user_data.block_number = new_block_number;
        user_data.processed_deposit_uuids.push(meta.uuid.clone());

        // save proof and user data
        self.store_vault_server
            .save_balance_proof(key.pubkey, &new_balance_proof)
            .await?;
        self.store_vault_server
            .save_user_data(key.pubkey, user_data.encrypt(key.pubkey))
            .await?;

        Ok(())
    }

    // sync deposit without updating the timestamp
    async fn sync_transfer(
        &self,
        key: KeySet,
        meta: &MetaData,
        transfer_data: &TransferData<F, C, D>,
    ) -> Result<(), SyncError> {
        log::info!("sync_transfer: {:?}", meta);
        if meta.block_number.is_none() {
            return Err(SyncError::InternalError(
                "block number is not set".to_string(),
            ));
        }
        let mut user_data = self.get_user_data(key).await?;
        // user's balance proof before applying the tx
        let prev_balance_proof = self
            .store_vault_server
            .get_balance_proof(
                key.pubkey,
                user_data.block_number,
                user_data.private_commitment(),
            )
            .await?;

        // sender balance proof after applying the tx
        let new_sender_balance_proof = match self
            .update_send_by_receiver(
                key,
                transfer_data.sender,
                meta.block_number.unwrap(),
                &transfer_data.tx_data,
            )
            .await
        {
            Ok(proof) => proof,
            Err(SyncError::SenderLastBlockNumberError {
                balance_proof_block_number,
                last_block_number,
            }) => {
                log::error!("Ignore tx: {} because of sender last block number error: balance_proof_block_number: {}, last_block_number: {}",meta.uuid, balance_proof_block_number, last_block_number);
                // TODO: handle this error
                return Ok(());
            }
            Err(e) => return Err(e),
        };

        let new_salt = generate_salt();
        let new_balance_proof = receive_transfer(
            &self.validity_prover,
            &self.balance_prover,
            key,
            &mut user_data.full_private_state,
            new_salt,
            &new_sender_balance_proof,
            &prev_balance_proof,
            &transfer_data,
        )
        .await?;

        let new_block_number = meta.block_number.unwrap().max(user_data.block_number);
        let new_balance_pis = BalancePublicInputs::from_pis(&new_balance_proof.public_inputs);
        if new_balance_pis.public_state.block_number != new_block_number {
            return Err(SyncError::BalanceProofBlockNumberMismatch {
                balance_proof_block_number: new_balance_pis.public_state.block_number,
                block_number: new_block_number,
            });
        }

        // update user data
        user_data.block_number = new_block_number;
        user_data.processed_transfer_uuids.push(meta.uuid.clone());

        // save proof and user data
        self.store_vault_server
            .save_balance_proof(key.pubkey, &new_balance_proof)
            .await?;
        self.store_vault_server
            .save_user_data(key.pubkey, user_data.encrypt(key.pubkey))
            .await?;

        Ok(())
    }

    async fn sync_withdrawal(
        &self,
        key: KeySet,
        meta: &MetaData,
        withdrawal_data: &TransferData<F, C, D>,
    ) -> Result<(), SyncError> {
        log::info!("sync_withdrawal: {:?}", meta);
        if meta.block_number.is_none() {
            return Err(SyncError::InternalError(
                "block number is not set".to_string(),
            ));
        }

        // get balance proof right after applying the tx
        let balance_proof = self
            .update_send_by_receiver(
                key,
                key.pubkey,
                meta.block_number.unwrap(),
                &withdrawal_data.tx_data,
            )
            .await?;

        let withdrawal_witness = WithdrawalWitness {
            transfer_witness: TransferWitness {
                transfer: withdrawal_data.transfer.clone(),
                transfer_index: withdrawal_data.transfer_index,
                transfer_merkle_proof: withdrawal_data.transfer_merkle_proof.clone(),
                tx: withdrawal_data.tx_data.tx.clone(),
            },
            balance_proof,
        };
        let single_withdrawal_proof = self
            .balance_prover
            .prove_single_withdrawal(key, &withdrawal_witness)
            .await?;

        // send withdrawal request
        self.withdrawal_server
            .request_withdrawal(key.pubkey, &single_withdrawal_proof)
            .await?;

        // update user data
        let mut user_data = self.get_user_data(key).await?;
        user_data.processed_withdrawal_uuids.push(meta.uuid.clone());
        self.store_vault_server
            .save_user_data(key.pubkey, user_data.encrypt(key.pubkey))
            .await?;

        Ok(())
    }

    async fn update_deposit_lpt(&self, key: KeySet, timestamp: u64) -> Result<(), SyncError> {
        log::info!("update_deposit_lpt: {:?}", timestamp);
        let mut user_data = self.get_user_data(key).await?;
        user_data.deposit_lpt = timestamp;
        self.store_vault_server
            .save_user_data(key.pubkey, user_data.encrypt(key.pubkey))
            .await?;
        Ok(())
    }

    async fn update_transfer_lpt(&self, key: KeySet, timestamp: u64) -> Result<(), SyncError> {
        log::info!("update_transfer_lpt: {:?}", timestamp);
        let mut user_data = self.get_user_data(key).await?;
        user_data.transfer_lpt = timestamp;
        self.store_vault_server
            .save_user_data(key.pubkey, user_data.encrypt(key.pubkey))
            .await?;
        Ok(())
    }

    async fn update_withdrawal_lpt(&self, key: KeySet, timestamp: u64) -> Result<(), SyncError> {
        log::info!("update_withdrawal_lpt: {:?}", timestamp);
        let mut user_data = self.get_user_data(key).await?;
        user_data.withdrawal_lpt = timestamp;
        self.store_vault_server
            .save_user_data(key.pubkey, user_data.encrypt(key.pubkey))
            .await?;
        Ok(())
    }

    async fn sync_tx(
        &self,
        key: KeySet,
        meta: &MetaData,
        tx_data: &TxData<F, C, D>,
    ) -> Result<(), SyncError> {
        log::info!("sync_tx: {:?}", meta);
        if meta.block_number.is_none() {
            return Err(SyncError::InternalError(
                "block number is not set".to_string(),
            ));
        }
        let mut user_data = self.get_user_data(key).await?;
        let prev_balance_proof = self
            .store_vault_server
            .get_balance_proof(
                key.pubkey,
                user_data.block_number,
                user_data.private_commitment(),
            )
            .await?;
        let balance_proof = update_send_by_sender(
            &self.validity_prover,
            &self.balance_prover,
            key,
            &mut user_data.full_private_state,
            &prev_balance_proof,
            meta.block_number.unwrap(),
            tx_data,
        )
        .await?;
        let balance_pis = BalancePublicInputs::from_pis(&balance_proof.public_inputs);
        if balance_pis.public_state.block_number != meta.block_number.unwrap() {
            return Err(SyncError::BalanceProofBlockNumberMismatch {
                balance_proof_block_number: balance_pis.public_state.block_number,
                block_number: meta.block_number.unwrap(),
            });
        }

        // update user data
        user_data.block_number = meta.block_number.unwrap();
        user_data.tx_lpt = meta.timestamp;
        user_data.processed_tx_uuids.push(meta.uuid.clone());

        // validation
        if balance_pis.private_commitment != user_data.private_commitment() {
            return Err(SyncError::InternalError(
                "private commitment mismatch".to_string(),
            ));
        }

        // save balance proof
        self.store_vault_server
            .save_balance_proof(key.pubkey, &balance_proof)
            .await?;
        // save user data
        self.store_vault_server
            .save_user_data(key.pubkey, user_data.encrypt(key.pubkey))
            .await?;
        Ok(())
    }

    // generate sender's balance proof after applying the tx
    // save the proof to the data store server
    async fn update_send_by_receiver(
        &self,
        key: KeySet,
        sender: U256,
        block_number: u32,
        common_tx_data: &CommonTxData<F, C, D>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, SyncError> {
        log::info!(
            "update_send_by_receiver: sender {}, block_number {}",
            sender,
            block_number
        );
        let spent_proof_pis =
            SpentPublicInputs::from_pis(&common_tx_data.spent_proof.public_inputs);

        let new_sender_balance_proof = self
            .store_vault_server
            .get_balance_proof(sender, block_number, spent_proof_pis.new_private_commitment)
            .await?;
        if new_sender_balance_proof.is_some() {
            // already updated
            return Ok(new_sender_balance_proof.unwrap());
        }

        let prev_sender_balance_proof = self
            .store_vault_server
            .get_balance_proof(
                sender,
                common_tx_data.sender_prev_block_number,
                spent_proof_pis.prev_private_commitment,
            )
            .await?
            .ok_or_else(|| SyncError::BalanceProofNotFound)?;

        let new_sender_balance_proof = update_send_by_receiver(
            &self.validity_prover,
            &self.balance_prover,
            key,
            sender,
            &Some(prev_sender_balance_proof),
            block_number,
            common_tx_data,
        )
        .await?;

        // save sender's balance proof
        self.store_vault_server
            .save_balance_proof(sender, &new_sender_balance_proof)
            .await?;

        Ok(new_sender_balance_proof)
    }

    async fn update_no_send(&self, key: KeySet, to_block_number: u32) -> Result<(), SyncError> {
        log::info!("update_no_send: {:?}", to_block_number);
        let mut user_data = self.get_user_data(key).await?;
        log::info!(
            "update_no_send: user_data.block_number {},  to_block_number {}",
            user_data.block_number,
            to_block_number
        );
        let prev_balance_proof = self
            .store_vault_server
            .get_balance_proof(
                key.pubkey,
                user_data.block_number,
                user_data.private_commitment(),
            )
            .await?;
        if user_data.block_number != 0 {
            if prev_balance_proof.is_none() {
                return Err(SyncError::BalanceProofNotFound);
            }
        }
        let new_balance_proof = update_no_send(
            &self.validity_prover,
            &self.balance_prover,
            key,
            &prev_balance_proof,
            to_block_number,
        )
        .await?;
        let new_balance_pis = BalancePublicInputs::from_pis(&new_balance_proof.public_inputs);
        let new_block_number = new_balance_pis.public_state.block_number;
        if new_block_number != to_block_number {
            return Err(SyncError::BalanceProofBlockNumberMismatch {
                balance_proof_block_number: new_block_number,
                block_number: to_block_number,
            });
        }

        // save balance proof
        self.store_vault_server
            .save_balance_proof(key.pubkey, &new_balance_proof)
            .await?;

        // update user data
        user_data.block_number = to_block_number;
        self.store_vault_server
            .save_user_data(key.pubkey, user_data.encrypt(key.pubkey))
            .await?;

        Ok(())
    }
}
