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
        transfer_data::TransferData,
        tx_data::TxData,
    },
};
use intmax2_zkp::{
    common::{
        block_builder::BlockProposal, deposit::get_pubkey_salt_hash, signature::key_set::KeySet,
        transfer::Transfer, trees::transfer_tree::TransferTree, tx::Tx,
        witness::spent_witness::SpentWitness,
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
    client::sync::utils::generate_salt,
    external_api::{
        contract::{liquidity_contract::LiquidityContract, rollup_contract::RollupContract},
        utils::time::sleep_for,
    },
};

use super::{
    config::ClientConfig,
    error::ClientError,
    history::{fetch_history, HistoryEntry},
    sync::balance_logic::generate_spent_witness,
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TxResult {
    pub tx_tree_root: Bytes32,
    pub transfer_data_vec: Vec<TransferData<F, C, D>>,
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
            return Err(ClientError::TransferLenError(
                "transfers is empty".to_string(),
            ));
        }
        if transfers.len() > NUM_TRANSFERS_IN_TX {
            return Err(ClientError::TransferLenError(
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
                .0
                .get(&transfer.token_index)
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

        // generate spent proof
        let tx_nonce = user_data.full_private_state.nonce;
        let spent_witness =
            generate_spent_witness(&user_data.full_private_state, tx_nonce, &transfers).await?;
        let spent_proof = self.balance_prover.prove_spent(key, &spent_witness).await?;
        let tx = spent_witness.tx;

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
                    if retries >= self.config.block_builder_request_limit {
                        return Err(ClientError::SendTxRequestError(format!(
                            "failed to send tx request: {}",
                            e
                        )));
                    }
                    retries += 1;
                    log::info!(
                        "Failed to send tx request, retrying in {} seconds. error: {}",
                        self.config.block_builder_request_interval,
                        e
                    );
                    sleep_for(self.config.block_builder_request_interval).await;
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
    ) -> Result<TxResult, ClientError> {
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

        let mut transfer_data_vec = Vec::new();
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
            transfer_data_vec.push(transfer_data);
        }

        let encrypted_transfer_data_vec = transfer_data_vec
            .iter()
            // filter out eth-address recipients (withdrawal)
            .filter(|data| data.transfer.recipient.is_pubkey)
            .map(|data| {
                let recipient = data.transfer.recipient.to_pubkey().unwrap();
                (recipient, data.encrypt(recipient))
            })
            .collect::<Vec<_>>();
        let encrypted_withdrawal_data_vec = transfer_data_vec
            .iter()
            // filter out pubkey recipients (transfer)
            .filter(|data| !data.transfer.recipient.is_pubkey)
            .map(|data| (key.pubkey, data.encrypt(key.pubkey)))
            .collect::<Vec<_>>();

        self.store_vault_server
            .save_data_batch(DataType::Transfer, encrypted_transfer_data_vec)
            .await?;
        self.store_vault_server
            .save_data_batch(DataType::Withdrawal, encrypted_withdrawal_data_vec)
            .await?;

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

        let result = TxResult {
            tx_tree_root: proposal.tx_tree_root,
            transfer_data_vec,
        };

        Ok(result)
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
