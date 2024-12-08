use async_trait::async_trait;
use intmax2_interfaces::api::{
    error::ServerError,
    validity_prover::{
        interface::{AccountInfo, DepositInfo, ValidityProverClientInterface},
        types::{
            GetAccountInfoQuery, GetAccountInfoResponse, GetBlockMerkleProofQuery,
            GetBlockMerkleProofResponse, GetBlockNumberByTxTreeRootQuery,
            GetBlockNumberByTxTreeRootResponse, GetBlockNumberResponse, GetDepositInfoQuery,
            GetDepositInfoResponse, GetDepositMerkleProofQuery, GetDepositMerkleProofResponse,
            GetSenderLeavesQuery, GetSenderLeavesResponse, GetUpdateWitnessQuery,
            GetUpdateWitnessResponse, GetValidityPisQuery, GetValidityPisResponse,
        },
    },
};
use intmax2_zkp::{
    circuits::validity::validity_pis::ValidityPublicInputs,
    common::{
        trees::{
            block_hash_tree::BlockHashMerkleProof, deposit_tree::DepositMerkleProof,
            sender_tree::SenderLeaf,
        },
        witness::update_witness::UpdateWitness,
    },
    ethereum_types::{bytes32::Bytes32, u256::U256},
};
use plonky2::{field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig};

use super::utils::query::get_request;

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Debug, Clone)]
pub struct ValidityProverClient {
    base_url: String,
}

impl ValidityProverClient {
    pub fn new(base_url: &str) -> Self {
        ValidityProverClient {
            base_url: base_url.to_string(),
        }
    }

    pub async fn sync(&self) -> Result<(), ServerError> {
        get_request::<(), ()>(&self.base_url, "/validity-prover/sync", None).await?;
        Ok(())
    }
}

#[async_trait(?Send)]
impl ValidityProverClientInterface for ValidityProverClient {
    async fn get_block_number(&self) -> Result<u32, ServerError> {
        let response: GetBlockNumberResponse =
            get_request::<(), _>(&self.base_url, "/validity-prover/block-number", None).await?;
        Ok(response.block_number)
    }

    async fn get_update_witness(
        &self,
        pubkey: U256,
        root_block_number: u32,
        leaf_block_number: u32,
        is_prev_account_tree: bool,
    ) -> Result<UpdateWitness<F, C, D>, ServerError> {
        let query = GetUpdateWitnessQuery {
            pubkey,
            root_block_number,
            leaf_block_number,
            is_prev_account_tree,
        };
        let response: GetUpdateWitnessResponse = get_request(
            &self.base_url,
            "/validity-prover/get-update-witness",
            Some(query),
        )
        .await?;
        Ok(response.update_witness)
    }

    async fn get_deposit_info(
        &self,
        deposit_hash: Bytes32,
    ) -> Result<Option<DepositInfo>, ServerError> {
        let query = GetDepositInfoQuery { deposit_hash };
        let response: GetDepositInfoResponse = get_request(
            &self.base_url,
            "/validity-prover/get-deposit-info",
            Some(query),
        )
        .await?;
        Ok(response.deposit_info)
    }

    async fn get_block_number_by_tx_tree_root(
        &self,
        tx_tree_root: Bytes32,
    ) -> Result<Option<u32>, ServerError> {
        let query = GetBlockNumberByTxTreeRootQuery { tx_tree_root };
        let response: GetBlockNumberByTxTreeRootResponse = get_request(
            &self.base_url,
            "/validity-prover/get-block-number-by-tx-tree-root",
            Some(query),
        )
        .await?;
        Ok(response.block_number)
    }

    async fn get_validity_pis(
        &self,
        block_number: u32,
    ) -> Result<Option<ValidityPublicInputs>, ServerError> {
        let query = GetValidityPisQuery { block_number };
        let response: GetValidityPisResponse = get_request(
            &self.base_url,
            "/validity-prover/get-validity-pis",
            Some(query),
        )
        .await?;
        Ok(response.validity_pis)
    }

    async fn get_sender_leaves(
        &self,
        block_number: u32,
    ) -> Result<Option<Vec<SenderLeaf>>, ServerError> {
        let query = GetSenderLeavesQuery { block_number };
        let response: GetSenderLeavesResponse = get_request(
            &self.base_url,
            "/validity-prover/get-sender-leaves",
            Some(query),
        )
        .await?;
        Ok(response.sender_leaves)
    }

    async fn get_block_merkle_proof(
        &self,
        root_block_number: u32,
        leaf_block_number: u32,
    ) -> Result<BlockHashMerkleProof, ServerError> {
        let query = GetBlockMerkleProofQuery {
            root_block_number,
            leaf_block_number,
        };
        let response: GetBlockMerkleProofResponse = get_request(
            &self.base_url,
            "/validity-prover/get-block-merkle-proof",
            Some(query),
        )
        .await?;
        Ok(response.block_merkle_proof)
    }

    async fn get_deposit_merkle_proof(
        &self,
        block_number: u32,
        deposit_index: u32,
    ) -> Result<DepositMerkleProof, ServerError> {
        let query = GetDepositMerkleProofQuery {
            block_number,
            deposit_index,
        };
        let response: GetDepositMerkleProofResponse = get_request(
            &self.base_url,
            "/validity-prover/get-deposit-merkle-proof",
            Some(query),
        )
        .await?;
        Ok(response.deposit_merkle_proof)
    }

    async fn get_account_info(&self, pubkey: U256) -> Result<AccountInfo, ServerError> {
        let query = GetAccountInfoQuery { pubkey };
        let response: GetAccountInfoResponse = get_request(
            &self.base_url,
            "/validity-prover/get-account-info",
            Some(query),
        )
        .await?;
        Ok(response.account_info)
    }
}
