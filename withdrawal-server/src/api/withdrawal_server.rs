use crate::api::{encode::encode_plonky2_proof, status::SqlWithdrawalStatus};

use super::error::WithdrawalServerError;
use intmax2_client_sdk::utils::circuit_verifiers::CircuitVerifiers;

use intmax2_interfaces::api::withdrawal_server::interface::WithdrawalInfo;
use intmax2_zkp::{
    common::{signature::flatten::FlatG2, withdrawal::Withdrawal},
    ethereum_types::{address::Address, u256::U256, u32limb_trait::U32LimbTrait},
    utils::conversion::ToU64,
};
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use sqlx::{postgres::PgPoolOptions, PgPool};

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

pub struct WithdrawalServer {
    pub pool: PgPool,
}

impl WithdrawalServer {
    pub async fn new(database_url: &str) -> anyhow::Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
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

    pub async fn get_withdrawal_info(
        &self,
        pubkey: U256,
        _signature: FlatG2,
    ) -> Result<Vec<WithdrawalInfo>, WithdrawalServerError> {
        // todo!: verify the signature
        let pubkey_str = pubkey.to_hex();
        let records = sqlx::query!(
            r#"
            SELECT 
                status as "status: SqlWithdrawalStatus",
                chained_withdrawal,
                withdrawal_id
            FROM withdrawal 
            WHERE pubkey = $1
            "#,
            pubkey_str
        )
        .fetch_all(&self.pool)
        .await?;

        let mut withdrawal_infos = Vec::new();
        for record in records {
            let withdrawal: Withdrawal = serde_json::from_value(record.chained_withdrawal)
                .map_err(|e| WithdrawalServerError::SerializationError(e.to_string()))?;
            withdrawal_infos.push(WithdrawalInfo {
                status: record.status.into(),
                withdrawal,
                withdrawal_id: record.withdrawal_id.map(|id| id as u32),
            });
        }
        Ok(withdrawal_infos)
    }

    pub async fn get_withdrawal_info_by_recipient(
        &self,
        recipient: Address,
    ) -> Result<Vec<WithdrawalInfo>, WithdrawalServerError> {
        let recipient_str = recipient.to_hex();
        let records = sqlx::query!(
            r#"
            SELECT 
                status as "status: SqlWithdrawalStatus",
                chained_withdrawal,
                withdrawal_id
            FROM withdrawal 
            WHERE recipient = $1
            "#,
            recipient_str
        )
        .fetch_all(&self.pool)
        .await?;

        let mut withdrawal_infos = Vec::new();
        for record in records {
            let withdrawal: Withdrawal = serde_json::from_value(record.chained_withdrawal)
                .map_err(|e| WithdrawalServerError::SerializationError(e.to_string()))?;
            withdrawal_infos.push(WithdrawalInfo {
                status: record.status.into(),
                withdrawal,
                withdrawal_id: record.withdrawal_id.map(|id| id as u32),
            });
        }
        Ok(withdrawal_infos)
    }
}
