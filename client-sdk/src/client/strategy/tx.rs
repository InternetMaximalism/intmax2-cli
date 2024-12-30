use intmax2_interfaces::{
    api::{
        store_vault_server::interface::{DataType, StoreVaultClientInterface},
        validity_prover::interface::ValidityProverClientInterface,
    },
    data::{meta_data::MetaData, tx_data::TxData},
};
use intmax2_zkp::common::signature::key_set::KeySet;
use plonky2::{
    field::{extension::Extendable, goldilocks_field::GoldilocksField},
    hash::hash_types::RichField,
    plonk::config::{GenericConfig, PoseidonGoldilocksConfig},
};

use super::error::StrategyError;

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Debug, Clone)]
pub struct TxInfo<F, C, const D: usize>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    pub settled: Vec<(MetaData, TxData<F, C, D>)>,
    pub pending: Vec<(MetaData, TxData<F, C, D>)>,
    pub timeout: Vec<(MetaData, TxData<F, C, D>)>,
}

pub async fn fetch_tx_info<S: StoreVaultClientInterface, V: ValidityProverClientInterface>(
    store_vault_server: &S,
    validity_prover: &V,
    key: KeySet,
    tx_lpt: u64,
    processed_tx_uuids: &[String],
    tx_timeout: u64,
) -> Result<TxInfo<F, C, D>, StrategyError> {
    let mut settled = Vec::new();
    let mut pending = Vec::new();
    let mut timeout = Vec::new();

    let encrypted_data = store_vault_server
        .get_data_all_after(DataType::Tx, key.pubkey, tx_lpt)
        .await?;
    for (meta, encrypted_data) in encrypted_data {
        if processed_tx_uuids.contains(&meta.uuid) {
            log::info!("Tx {} is already processed", meta.uuid);
            continue;
        }
        match TxData::decrypt(&encrypted_data, key) {
            Ok(tx_data) => {
                let tx_tree_root = tx_data.common.tx_tree_root;
                let block_number = validity_prover
                    .get_block_number_by_tx_tree_root(tx_tree_root)
                    .await?;
                if let Some(block_number) = block_number {
                    // set block number
                    let mut meta = meta;
                    meta.block_number = Some(block_number);
                    settled.push((meta, tx_data));
                } else {
                    if meta.timestamp + tx_timeout < chrono::Utc::now().timestamp() as u64 {
                        // timeout
                        log::error!("Tx {} is timeout", meta.uuid);
                        timeout.push((meta, tx_data));
                    } else {
                        // pending
                        log::info!("Tx {} is pending", meta.uuid);
                        pending.push((meta, tx_data));
                    }
                }
            }
            Err(e) => {
                // just ignore the invalid data
                log::error!("failed to decrypt tx data: {}", e);
            }
        };
    }

    // sort by block number
    settled.sort_by_key(|(meta, _)| meta.block_number.unwrap());

    // sort by timestamp
    pending.sort_by_key(|(meta, _)| meta.timestamp);

    Ok(TxInfo {
        settled,
        pending,
        timeout,
    })
}
