use intmax2_interfaces::{
    api::{
        balance_prover::interface::BalanceProverClientInterface,
        block_builder::interface::BlockBuilderClientInterface,
        store_vault_server::interface::{DataType, StoreVaultClientInterface},
        validity_prover::interface::ValidityProverClientInterface,
        withdrawal_server::interface::{WithdrawalInfo, WithdrawalServerClientInterface},
    },
    data::{
        common_tx_data::CommonTxData,
        deposit_data::{DepositData, TokenType},
        meta_data::MetaData,
        transfer_data::TransferData,
        tx_data::TxData,
        user_data::UserData,
    },
};
use intmax2_zkp::{
    circuits::balance::{balance_pis::BalancePublicInputs, send::spent_circuit::SpentPublicInputs},
    common::{
        block_builder::BlockProposal,
        deposit::get_pubkey_salt_hash,
        signature::key_set::KeySet,
        transfer::Transfer,
        trees::transfer_tree::TransferTree,
        tx::Tx,
        witness::{
            spent_witness::SpentWitness, transfer_witness::TransferWitness,
            withdrawal_witness::WithdrawalWitness,
        },
    },
    constants::{NUM_TRANSFERS_IN_TX, TRANSFER_TREE_HEIGHT},
    ethereum_types::{address::Address, bytes32::Bytes32, u256::U256},
    utils::poseidon_hash_out::PoseidonHashOut,
};

use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use serde::{Deserialize, Serialize};

use crate::{
    client::{
        balance_logic::{process_common_tx, process_transfer},
        utils::generate_salt,
    },
    external_api::contract::{
        liquidity_contract::LiquidityContract, rollup_contract::RollupContract,
    },
};

use super::{
    balance_logic::process_deposit,
    config::ClientConfig,
    error::ClientError,
    history::{fetch_history, HistoryEntry},
    strategy::{
        strategy::{determin_next_action, Action},
        withdrawal::fetch_withdrawal_info,
    },
    utils::generate_transfer_tree,
};

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

pub struct Client<
    BB: BlockBuilderClientInterface,
    S: StoreVaultClientInterface,
    V: ValidityProverClientInterface,
    B: BalanceProverClientInterface,
    W: WithdrawalServerClientInterface,
> {
    pub config: ClientConfig,

    pub block_builder: BB,
    pub store_vault_server: S,
    pub validity_prover: V,
    pub balance_prover: B,
    pub withdrawal_server: W,

    pub liquidity_contract: LiquidityContract,
    pub rollup_contract: RollupContract,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SyncStatus {
    Continue, // continue syncing
    Complete, // sync completed
    Pending,  // there are pending actions
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TxRequestMemo {
    pub is_registration_block: bool,
    pub tx: Tx,
    pub transfers: Vec<Transfer>,
    pub spent_witness: SpentWitness,
    pub spent_proof: ProofWithPublicInputs<F, C, D>,
    pub prev_block_number: u32,
    pub prev_private_commitment: PoseidonHashOut,
}

impl<BB, S, V, B, W> Client<BB, S, V, B, W>
where
    BB: BlockBuilderClientInterface,
    S: StoreVaultClientInterface,
    V: ValidityProverClientInterface,
    B: BalanceProverClientInterface,
    W: WithdrawalServerClientInterface,
{
    /// Back up deposit information before calling the contract's deposit function
    pub async fn prepare_deposit(
        &self,
        pubkey: U256,
        amount: U256,
        token_type: TokenType,
        token_address: Address,
        token_id: U256,
    ) -> Result<DepositData, ClientError> {
        log::info!(
            "prepare_deposit: pubkey {}, amount {}, token_type {:?}, token_address {}, token_id {}",
            pubkey,
            amount,
            token_type,
            token_address,
            token_id
        );
        let deposit_salt = generate_salt();

        // backup before contract call
        let pubkey_salt_hash = get_pubkey_salt_hash(pubkey, deposit_salt);
        let deposit_data = DepositData {
            deposit_salt,
            pubkey_salt_hash,
            amount,
            token_type,
            token_address,
            token_id,
            token_index: None,
        };
        self.store_vault_server
            .save_data(DataType::Deposit, pubkey, &deposit_data.encrypt(pubkey))
            .await?;

        Ok(deposit_data)
    }

    /// Send a transaction request to the block builder
    pub async fn send_tx_request(
        &self,
        block_builder_url: &str,
        key: KeySet,
        transfers: Vec<Transfer>,
    ) -> Result<TxRequestMemo, ClientError> {
        // input validation
        if transfers.len() == 0 {
            return Err(ClientError::InternalError("transfers is empty".to_string()));
        }
        if transfers.len() > NUM_TRANSFERS_IN_TX {
            return Err(ClientError::InternalError(
                "transfers is too long".to_string(),
            ));
        }

        // sync balance proof
        self.sync(key).await?;

        let user_data = self.get_user_data(key).await?;

        // balance check
        let balances = user_data.balances();
        for transfer in &transfers {
            let balance = balances
                .get(&(transfer.token_index as u64))
                .cloned()
                .unwrap_or_default();
            if balance.is_insufficient {
                return Err(ClientError::BalanceError(format!(
                    "Already insufficient: token index {}",
                    transfer.token_index
                )));
            }
            if balance.amount < transfer.amount {
                return Err(ClientError::BalanceError(format!(
                    "Insufficient balance: {} < {} for token #{}",
                    balance.amount, transfer.amount, transfer.token_index
                )));
            }
        }

        // balance proof existence check
        let _balance_proof = self
            .store_vault_server
            .get_balance_proof(
                key.pubkey,
                user_data.block_number,
                user_data.private_commitment(),
            )
            .await?
            .ok_or_else(|| ClientError::BalanceProofNotFound)?;

        // generate spent proof
        let transfer_tree = generate_transfer_tree(&transfers);
        let tx = Tx {
            nonce: user_data.full_private_state.nonce,
            transfer_tree_root: transfer_tree.get_root(),
        };
        let new_salt = generate_salt();
        let spent_witness = SpentWitness::new(
            &user_data.full_private_state.asset_tree,
            &user_data.full_private_state.to_private_state(),
            &transfer_tree.leaves(), // this is padded
            tx,
            new_salt,
        )
        .map_err(|e| {
            ClientError::WitnessGenerationError(format!("failed to generate spent witness: {}", e))
        })?;
        let spent_proof = self.balance_prover.prove_spent(key, &spent_witness).await?;

        // fetch if this is first time tx
        let account_info = self.validity_prover.get_account_info(key.pubkey).await?;
        let is_registration_block = account_info.account_id.is_none();

        // send tx request
        let mut retries = 0;
        loop {
            let result = self
                .block_builder
                .send_tx_request(
                    block_builder_url,
                    is_registration_block,
                    key.pubkey,
                    tx,
                    None,
                )
                .await;
            match result {
                Ok(_) => break,
                Err(e) => {
                    if retries >= self.config.max_tx_request_retries {
                        return Err(ClientError::SendTxRequestError(format!(
                            "failed to send tx request: {}",
                            e
                        )));
                    }
                    retries += 1;
                    log::info!(
                        "Failed to send tx request, retrying in {} seconds. error: {}",
                        self.config.tx_request_retry_interval,
                        e
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(
                        self.config.tx_request_retry_interval,
                    ))
                    .await;
                }
            }
        }

        let memo = TxRequestMemo {
            is_registration_block,
            tx,
            transfers,
            spent_witness,
            spent_proof,
            prev_block_number: user_data.block_number,
            prev_private_commitment: user_data.private_commitment(),
        };
        Ok(memo)
    }

    pub async fn query_proposal(
        &self,
        block_builder_url: &str,
        key: KeySet,
        is_registration_block: bool,
        tx: Tx,
    ) -> Result<Option<BlockProposal>, ClientError> {
        let proposal = self
            .block_builder
            .query_proposal(block_builder_url, is_registration_block, key.pubkey, tx)
            .await?;
        Ok(proposal)
    }

    /// Verify the proposal, and send the signature to the block builder
    pub async fn finalize_tx(
        &self,
        block_builder_url: &str,
        key: KeySet,
        memo: &TxRequestMemo,
        proposal: &BlockProposal,
    ) -> Result<Bytes32, ClientError> {
        // verify proposal
        proposal
            .verify(memo.tx)
            .map_err(|e| ClientError::InvalidBlockProposal(format!("{}", e)))?;

        // backup before posting signature
        let common_tx_data = CommonTxData {
            spent_proof: memo.spent_proof.clone(),
            sender_prev_block_number: memo.prev_block_number,
            tx: memo.tx.clone(),
            tx_index: proposal.tx_index,
            tx_merkle_proof: proposal.tx_merkle_proof.clone(),
            tx_tree_root: proposal.tx_tree_root,
        };

        // save tx data
        let tx_data = TxData {
            common: common_tx_data.clone(),
            spent_witness: memo.spent_witness.clone(),
        };
        self.store_vault_server
            .save_data(DataType::Tx, key.pubkey, &tx_data.encrypt(key.pubkey))
            .await?;

        // save transfer data
        let mut transfer_tree = TransferTree::new(TRANSFER_TREE_HEIGHT);
        for transfer in &memo.transfers {
            transfer_tree.push(transfer.clone());
        }
        for (i, transfer) in memo.transfers.iter().enumerate() {
            let transfer_merkle_proof = transfer_tree.prove(i as u64);
            let transfer_data = TransferData {
                sender: key.pubkey,
                prev_block_number: memo.prev_block_number,
                prev_private_commitment: memo.prev_private_commitment,
                tx_data: common_tx_data.clone(),
                transfer: transfer.clone(),
                transfer_index: i as u32,
                transfer_merkle_proof,
            };
            if transfer.recipient.is_pubkey {
                let recipient = transfer.recipient.to_pubkey().unwrap();
                self.store_vault_server
                    .save_data(
                        DataType::Transfer,
                        transfer.recipient.to_pubkey().unwrap(),
                        &transfer_data.encrypt(recipient),
                    )
                    .await?;
            } else {
                self.store_vault_server
                    .save_data(
                        DataType::Withdrawal,
                        key.pubkey,
                        &transfer_data.encrypt(key.pubkey),
                    )
                    .await?;
            }
        }

        // sign and post signature
        let signature = proposal.sign(key);
        self.block_builder
            .post_signature(
                block_builder_url,
                memo.is_registration_block,
                signature.pubkey,
                memo.tx,
                signature.signature,
            )
            .await?;

        Ok(proposal.tx_tree_root)
    }

    /// Sync the client's balance proof with the latest block
    pub async fn sync(&self, key: KeySet) -> Result<(), ClientError> {
        let mut sync_status = SyncStatus::Continue;
        while sync_status == SyncStatus::Continue {
            sync_status = self.sync_single(key).await?;
        }
        if sync_status == SyncStatus::Pending {
            return Err(ClientError::PendingError(
                "there is pending actions".to_string(),
            ));
        }
        Ok(())
    }

    pub async fn sync_single(&self, key: KeySet) -> Result<SyncStatus, ClientError> {
        let next_action = determin_next_action(
            &self.store_vault_server,
            &self.validity_prover,
            &self.liquidity_contract,
            key,
            self.config.deposit_timeout,
            self.config.tx_timeout,
        )
        .await?;

        // if there are pending actions, return pending
        // todo: process non-pending actions if possible
        if next_action.pending_deposits.len() > 0
            || next_action.pending_transfers.len() > 0
            || next_action.pending_txs.len() > 0
        {
            return Ok(SyncStatus::Pending);
        }

        if next_action.action.is_none() {
            return Ok(SyncStatus::Complete);
        }

        match next_action.action.unwrap() {
            Action::Deposit(meta, deposit_data) => {
                self.sync_deposit(key, &meta, &deposit_data).await?;
            }
            Action::Transfer(meta, transfer_data) => {
                self.sync_transfer(key, &meta, &transfer_data).await?;
            }
            Action::Tx(meta, tx_data) => self.sync_tx(key, &meta, &tx_data).await?,
        }

        Ok(SyncStatus::Continue)
    }

    pub async fn sync_withdrawals(&self, key: KeySet) -> Result<(), ClientError> {
        // sync balance proof
        self.sync(key).await?;

        let user_data = self.get_user_data(key).await?;

        let withdrawal_info = fetch_withdrawal_info(
            &self.store_vault_server,
            &self.validity_prover,
            key,
            user_data.withdrawal_lpt,
            self.config.tx_timeout,
        )
        .await?;
        if withdrawal_info.pending.len() > 0 {
            return Err(ClientError::PendingError("pending withdrawals".to_string()));
        }
        for (meta, data) in &withdrawal_info.settled {
            self.sync_withdrawal(key, meta, data).await?;
        }
        Ok(())
    }

    async fn sync_deposit(
        &self,
        key: KeySet,
        meta: &MetaData,
        deposit_data: &DepositData,
    ) -> Result<(), ClientError> {
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
        let new_balance_proof = process_deposit(
            &self.validity_prover,
            &self.balance_prover,
            key,
            user_data.pubkey,
            &mut user_data.full_private_state,
            new_salt,
            &prev_balance_proof,
            meta.block_number.unwrap(),
            deposit_data,
        )
        .await?;

        // update user data
        user_data.block_number = meta.block_number.unwrap();
        user_data.deposit_lpt = meta.timestamp;
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

    async fn sync_transfer(
        &self,
        key: KeySet,
        meta: &MetaData,
        transfer_data: &TransferData<F, C, D>,
    ) -> Result<(), ClientError> {
        log::info!("sync_transfer: {:?}", meta);
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
        let new_sender_balance_proof = self
            .generate_new_sender_balance_proof(
                key,
                transfer_data.sender,
                meta.block_number.unwrap(),
                &transfer_data.tx_data,
            )
            .await?;

        let new_salt = generate_salt();
        let new_balance_proof = process_transfer(
            &self.validity_prover,
            &self.balance_prover,
            key,
            user_data.pubkey,
            &mut user_data.full_private_state,
            new_salt,
            &new_sender_balance_proof,
            &prev_balance_proof,
            meta.block_number.unwrap(),
            &transfer_data,
        )
        .await?;

        // update user data
        user_data.block_number = meta.block_number.unwrap();
        user_data.transfer_lpt = meta.timestamp;
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

    async fn sync_tx(
        &self,
        key: KeySet,
        meta: &MetaData,
        tx_data: &TxData<F, C, D>,
    ) -> Result<(), ClientError> {
        log::info!("sync_tx: {:?}", meta);
        let mut user_data = self.get_user_data(key).await?;
        let balance_proof = self
            .generate_new_sender_balance_proof(
                key,
                key.pubkey,
                meta.block_number.unwrap(),
                &tx_data.common,
            )
            .await?;
        let balance_pis = BalancePublicInputs::from_pis(&balance_proof.public_inputs);
        if balance_pis.public_state.block_number != meta.block_number.unwrap() {
            return Err(ClientError::SyncError("block number mismatch".to_string()));
        }

        // update user data
        user_data.block_number = meta.block_number.unwrap();
        user_data.tx_lpt = meta.timestamp;
        user_data.processed_tx_uuids.push(meta.uuid.clone());
        tx_data
            .spent_witness
            .update_private_state(&mut user_data.full_private_state)
            .map_err(|e| {
                ClientError::InternalError(format!("failed to update private state: {}", e))
            })?;

        // validation
        if balance_pis.private_commitment != user_data.private_commitment() {
            return Err(ClientError::InternalError(
                "private commitment mismatch".to_string(),
            ));
        }

        // save user data
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
    ) -> Result<(), ClientError> {
        log::info!("sync_withdrawal: {:?}", meta);
        if meta.block_number.is_none() {
            return Err(ClientError::InternalError(
                "block number is not set".to_string(),
            ));
        }

        let mut user_data = self.get_user_data(key).await?;

        let new_user_balance_proof = self
            .generate_new_sender_balance_proof(
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
            balance_proof: new_user_balance_proof,
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
        user_data.block_number = meta.block_number.unwrap();
        user_data.withdrawal_lpt = meta.timestamp;
        user_data.processed_withdrawal_uuids.push(meta.uuid.clone());

        // save user data
        self.store_vault_server
            .save_user_data(key.pubkey, user_data.encrypt(key.pubkey))
            .await?;

        Ok(())
    }

    // generate sender's balance proof after applying the tx
    // save the proof to the data store server
    async fn generate_new_sender_balance_proof(
        &self,
        key: KeySet,
        sender: U256,
        block_number: u32,
        common_tx_data: &CommonTxData<F, C, D>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ClientError> {
        log::info!(
            "generate_new_sender_balance_proof: sender {}, block_number {}",
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
            .ok_or_else(|| ClientError::BalanceProofNotFound)?;

        let new_sender_balance_proof = process_common_tx(
            &self.validity_prover,
            &self.balance_prover,
            key,
            sender,
            &Some(prev_sender_balance_proof),
            block_number,
            common_tx_data,
        )
        .await?;

        self.store_vault_server
            .save_balance_proof(sender, &new_sender_balance_proof)
            .await?;

        Ok(new_sender_balance_proof)
    }

    /// Get the latest user data from the data store server
    pub async fn get_user_data(&self, key: KeySet) -> Result<UserData, ClientError> {
        let user_data = self
            .store_vault_server
            .get_user_data(key.pubkey)
            .await?
            .map(|encrypted| UserData::decrypt(&encrypted, key))
            .transpose()
            .map_err(|e| {
                ClientError::DecryptionError(format!("failed to decrypt user data: {}", e))
            })?
            .unwrap_or(UserData::new(key.pubkey));
        Ok(user_data)
    }

    pub async fn get_withdrawal_info(
        &self,
        key: KeySet,
    ) -> Result<Vec<WithdrawalInfo>, ClientError> {
        let withdrawal_info = self.withdrawal_server.get_withdrawal_info(key).await?;
        Ok(withdrawal_info)
    }

    pub async fn fetch_history(&self, key: KeySet) -> Result<Vec<HistoryEntry>, ClientError> {
        fetch_history(self, key).await
    }
}
