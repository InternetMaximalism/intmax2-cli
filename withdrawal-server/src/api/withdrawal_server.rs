use crate::api::{encode::encode_plonky2_proof, status::SqlWithdrawalStatus};

use super::error::WithdrawalServerError;
use intmax2_client_sdk::utils::circuit_verifiers::CircuitVerifiers;

use intmax2_zkp::{
    common::withdrawal::Withdrawal,
    ethereum_types::{u256::U256, u32limb_trait::U32LimbTrait},
    utils::conversion::ToU64,
};
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use sqlx::PgPool;

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

pub struct WithdrawalServer {
    pub pool: PgPool,
}

impl WithdrawalServer {
    pub async fn new(database_url: &str) -> anyhow::Result<Self> {
        let pool = PgPool::connect(database_url).await?;
        Ok(Self { pool })
    }

    pub async fn request_withdrawal(
        &self,
        pubkey: U256,
        single_withdrawal_proof: &ProofWithPublicInputs<F, C, D>,
    ) -> Result<(), WithdrawalServerError> {
        // Verify the single withdrawal proof
        let single_withdrawal_vd = CircuitVerifiers::load().get_single_withdrawal_vd();
        single_withdrawal_vd
            .verify(single_withdrawal_proof.clone())
            .map_err(|_| WithdrawalServerError::SingleWithdrawalVerificationError)?;

        // Serialize the proof and public inputs
        let proof_bytes =
            encode_plonky2_proof(single_withdrawal_proof.clone(), &single_withdrawal_vd)
                .map_err(|e| WithdrawalServerError::SerializationError(e.to_string()))?;
        let withdrawal =
            Withdrawal::from_u64_slice(&single_withdrawal_proof.public_inputs.to_u64_vec());
        let pubkey_str = pubkey.to_hex();
        let recipient = withdrawal.recipient.to_hex();
        let chained_withdrawal = serde_json::to_value(withdrawal)
            .map_err(|e| WithdrawalServerError::SerializationError(e.to_string()))?;

        sqlx::query!(
            r#"
            INSERT INTO withdrawal (
                pubkey,
                recipient,
                single_withdrawal_proof,
                chained_withdrawal,
                status
            )
           VALUES ($1, $2, $3, $4, $5::withdrawal_status)
            "#,
            pubkey_str,
            recipient,
            proof_bytes,
            chained_withdrawal,
            SqlWithdrawalStatus::Requested as SqlWithdrawalStatus
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_withdrawal_info() -> Result<(), WithdrawalServerError> {
        todo!()
    }
}
