use intmax2_client_sdk::client::client::Client;
use intmax2_interfaces::api::{
    balance_prover::interface::BalanceProverClientInterface,
    block_builder::interface::BlockBuilderClientInterface,
    store_vault_server::interface::StoreVaultClientInterface,
    validity_prover::interface::ValidityProverClientInterface,
    withdrawal_server::interface::WithdrawalServerClientInterface,
};
use intmax2_zkp::{
    circuits::validity::validity_pis::ValidityPublicInputs,
    common::trees::block_hash_tree::BlockHashMerkleProof,
};

#[derive(Debug, Clone)]
pub struct ValidationData {
    pub latest_validity_pis: ValidityPublicInputs,
    pub deposit_validity_pis: ValidityPublicInputs,
    pub deposit_block_merkle_proof: BlockHashMerkleProof,
    pub withdrawal_validity_pis: ValidityPublicInputs,
    pub withdrawal_block_merkle_proof: BlockHashMerkleProof,
}

impl Default for ValidationData {
    fn default() -> Self {
        Self {
            latest_validity_pis: ValidityPublicInputs::genesis(),
            deposit_validity_pis: ValidityPublicInputs::genesis(),
            deposit_block_merkle_proof: BlockHashMerkleProof::dummy(32),
            withdrawal_validity_pis: ValidityPublicInputs::genesis(),
            withdrawal_block_merkle_proof: BlockHashMerkleProof::dummy(32),
        }
    }
}

pub async fn fetch_validation_data<
    BB: BlockBuilderClientInterface,
    S: StoreVaultClientInterface,
    V: ValidityProverClientInterface,
    B: BalanceProverClientInterface,
    W: WithdrawalServerClientInterface,
>(
    client: &Client<BB, S, V, B, W>,
    processed_deposit_block: u32,
    processed_withdrawal_block: u32,
) -> anyhow::Result<ValidationData> {
    let latest_block_number = client.validity_prover.get_block_number().await?;
    let deposit_block_merkle_proof = client
        .validity_prover
        .get_block_merkle_proof(latest_block_number, processed_deposit_block)
        .await?;
    let withdrawal_block_merkle_proof = client
        .validity_prover
        .get_block_merkle_proof(latest_block_number, processed_withdrawal_block)
        .await?;
    let latest_validity_pis = client
        .validity_prover
        .get_validity_pis(latest_block_number)
        .await?
        .unwrap();
    let deposit_validity_pis = client
        .validity_prover
        .get_validity_pis(processed_deposit_block)
        .await?
        .unwrap();
    let withdrawal_validity_pis = client
        .validity_prover
        .get_validity_pis(processed_withdrawal_block)
        .await?
        .unwrap();

    Ok(ValidationData {
        latest_validity_pis,
        deposit_validity_pis,
        deposit_block_merkle_proof,
        withdrawal_validity_pis,
        withdrawal_block_merkle_proof,
    })
}
