use intmax2_interfaces::{
    api::{
        store_vault_server::interface::{DataType, StoreVaultClientInterface},
        validity_prover::interface::ValidityProverClientInterface,
    },
    data::{deposit_data::DepositData, meta_data::MetaData},
};
use intmax2_zkp::common::signature::key_set::KeySet;

use crate::external_api::contract::liquidity_contract::LiquidityContract;

use super::error::StrategyError;

#[derive(Debug, Clone)]
pub struct DepositInfo {
    pub settled: Vec<(MetaData, DepositData)>,
    pub pending: Vec<(MetaData, DepositData)>,
    pub timeout: Vec<(MetaData, DepositData)>,
}

pub async fn fetch_deposit_info<S: StoreVaultClientInterface, V: ValidityProverClientInterface>(
    store_vault_server: &S,
    validity_prover: &V,
    liquidity_contract: &LiquidityContract,
    key: KeySet,
    deposit_lpt: u64,
    processed_deposit_uuids: &[String],
    deposit_timeout: u64,
) -> Result<DepositInfo, StrategyError> {
    let mut settled = Vec::new();
    let mut pending = Vec::new();
    let mut timeout = Vec::new();

    let encrypted_data = store_vault_server
        .get_data_all_after(DataType::Deposit, key.pubkey, deposit_lpt)
        .await?;
    for (meta, encrypted_data) in encrypted_data {
        if processed_deposit_uuids.contains(&meta.uuid) {
            log::info!("Deposit {} is already processed", meta.uuid);
            continue;
        }
        match DepositData::decrypt(&encrypted_data, key) {
            Ok(deposit_data) => {
                let token_index = liquidity_contract
                    .get_token_index(
                        deposit_data.token_type,
                        deposit_data.token_address,
                        deposit_data.token_id,
                    )
                    .await?;
                if token_index.is_none() {
                    log::error!("Token not found: {:?}", deposit_data);
                    // ignore this deposit
                    continue;
                }
                let mut deposit_data = deposit_data;
                deposit_data.set_token_index(token_index.unwrap());
                let deposit_hash = deposit_data.deposit_hash().unwrap();

                if let Some(deposit_info) = validity_prover.get_deposit_info(deposit_hash).await? {
                    // set block number
                    let mut meta = meta;
                    meta.block_number = Some(deposit_info.block_number);
                    settled.push((meta, deposit_data));
                } else {
                    if meta.timestamp + deposit_timeout < chrono::Utc::now().timestamp() as u64 {
                        // timeout
                        log::error!(
                            "Deposit uuid: {}, hash: {} is timeout",
                            meta.uuid,
                            deposit_hash
                        );
                        timeout.push((meta, deposit_data));
                    } else {
                        // pending
                        log::info!("Deposit {} is pending", meta.uuid);
                        pending.push((meta, deposit_data));
                    }
                }
            }
            Err(e) => {
                log::error!("failed to decrypt deposit data: {}", e);
                // ignore this deposit
            }
        };
    }

    // sort by block number
    settled.sort_by_key(|(meta, _)| meta.block_number.unwrap());

    Ok(DepositInfo {
        settled,
        pending,
        timeout,
    })
}
