use anyhow::{Ok, Result};
use intmax2_interfaces::{api::store_vault_server::interface::DataType, data::meta_data::MetaData};
use intmax2_zkp::{
    circuits::balance::balance_pis::BalancePublicInputs,
    ethereum_types::{u256::U256, u32limb_trait::U32LimbTrait},
    utils::poseidon_hash_out::PoseidonHashOut,
};
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

pub struct StoreVaultServer {
    pool: PgPool,
}

impl StoreVaultServer {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        Ok(Self { pool })
    }

    pub async fn reset(&mut self) -> Result<()> {
        sqlx::query!("TRUNCATE encrypted_user_data, balance_proofs, encrypted_data")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn save_balance_proof(
        &mut self,
        pubkey: U256,
        proof: ProofWithPublicInputs<F, C, D>,
    ) -> Result<()> {
        let balance_pis = BalancePublicInputs::from_pis(&proof.public_inputs);
        let pubkey_hex = pubkey.to_hex();
        let private_commitment_hex = format!("{}", balance_pis.private_commitment);

        let proof_data = bincode::serialize(&proof)?;

        sqlx::query!(
            r#"
            INSERT INTO balance_proofs (pubkey, block_number, private_commitment, proof_data)
            VALUES ($1, $2, $3, $4)
            "#,
            pubkey_hex,
            balance_pis.public_state.block_number as i32,
            private_commitment_hex,
            proof_data
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_balance_proof(
        &self,
        pubkey: U256,
        block_number: u32,
        private_commitment: PoseidonHashOut,
    ) -> Result<Option<ProofWithPublicInputs<F, C, D>>> {
        let pubkey_hex = pubkey.to_hex();
        let private_commitment_hex = format!("{}", private_commitment);

        let record = sqlx::query!(
            r#"
            SELECT proof_data
            FROM balance_proofs
            WHERE pubkey = $1 AND block_number = $2 AND private_commitment = $3
            "#,
            pubkey_hex,
            block_number as i32,
            private_commitment_hex
        )
        .fetch_optional(&self.pool)
        .await?;

        match record {
            Some(record) => {
                let proof: ProofWithPublicInputs<F, C, D> =
                    bincode::deserialize(&record.proof_data)?;
                Ok(Some(proof))
            }
            None => Ok(None),
        }
    }

    pub async fn save_user_data(&mut self, pubkey: U256, encrypted_data: Vec<u8>) -> Result<()> {
        let pubkey_hex = pubkey.to_hex();

        sqlx::query!(
            r#"
            INSERT INTO encrypted_user_data (pubkey, encrypted_data)
            VALUES ($1, $2)
            ON CONFLICT (pubkey) DO UPDATE SET encrypted_data = EXCLUDED.encrypted_data
            "#,
            pubkey_hex,
            encrypted_data
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_user_data(&self, pubkey: U256) -> Result<Option<Vec<u8>>> {
        let pubkey_hex = pubkey.to_hex();

        let record = sqlx::query!(
            r#"
            SELECT encrypted_data FROM encrypted_user_data WHERE pubkey = $1
            "#,
            pubkey_hex
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(record.map(|r| r.encrypted_data))
    }

    pub async fn save_data(
        &mut self,
        data_type: DataType,
        pubkey: U256,
        encrypted_data: Vec<u8>,
    ) -> Result<()> {
        let pubkey_hex = pubkey.to_hex();
        let uuid = Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().timestamp() as i64;

        sqlx::query!(
            r#"
            INSERT INTO encrypted_data 
            (data_type, pubkey, uuid, timestamp, encrypted_data)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            data_type as i32,
            pubkey_hex,
            uuid,
            timestamp,
            encrypted_data
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_data_all_after(
        &self,
        data_type: DataType,
        pubkey: U256,
        timestamp: u64,
    ) -> Result<Vec<(MetaData, Vec<u8>)>> {
        let pubkey_hex = pubkey.to_hex();

        let records = sqlx::query!(
            r#"
            SELECT uuid, timestamp, block_number, encrypted_data
            FROM encrypted_data
            WHERE data_type = $1 AND pubkey = $2 AND timestamp > $3
            ORDER BY timestamp ASC
            "#,
            data_type as i32,
            pubkey_hex,
            timestamp as i64
        )
        .fetch_all(&self.pool)
        .await?;

        let result = records
            .into_iter()
            .map(|r| {
                let meta_data = MetaData {
                    uuid: r.uuid,
                    timestamp: r.timestamp as u64,
                    block_number: r.block_number.map(|n| n as u32),
                };
                (meta_data, r.encrypted_data)
            })
            .collect();

        Ok(result)
    }

    pub async fn get_data(
        &self,
        data_type: DataType,
        uuid: &str,
    ) -> Result<Option<(MetaData, Vec<u8>)>> {
        let record = sqlx::query!(
            r#"
            SELECT timestamp, block_number, encrypted_data
            FROM encrypted_data
            WHERE data_type = $1 AND uuid = $2
            "#,
            data_type as i32,
            uuid
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(record.map(|r| {
            let meta_data = MetaData {
                uuid: uuid.to_string(),
                timestamp: r.timestamp as u64,
                block_number: r.block_number.map(|n| n as u32),
            };
            (meta_data, r.encrypted_data)
        }))
    }
}
