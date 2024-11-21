use intmax2_interfaces::{
    api::{
        store_vault_server::interface::{DataType, StoreVaultClientInterface},
        validity_prover::interface::ValidityProverClientInterface,
    },
    data::{meta_data::MetaData, transfer_data::TransferData},
};
use intmax2_zkp::common::signature::key_set::KeySet;
use plonky2::{field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig};

use crate::client::error::ClientError;

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Debug, Clone)]
pub struct TransferInfo {
    pub settled: Vec<(MetaData, TransferData<F, C, D>)>,
    pub pending: Vec<MetaData>,
    pub rejected: Vec<MetaData>,
}

pub async fn fetch_transfer_info<S: StoreVaultClientInterface, V: ValidityProverClientInterface>(
    store_vault_server: &S,
    validity_prover: &V,
    key: KeySet,
    transfer_lpt: u64,
    tx_timeout: u64,
) -> Result<TransferInfo, ClientError> {
    let mut settled = Vec::new();
    let mut pending = Vec::new();
    let mut rejected = Vec::new();

    let encrypted_data = store_vault_server
        .get_data_all_after(DataType::Transfer, key.pubkey, transfer_lpt)
        .await?;
    for (meta, encrypted_data) in encrypted_data {
        match TransferData::decrypt(&encrypted_data, key) {
            Ok(transfer_data) => {
                let tx_tree_root = transfer_data.tx_data.tx_tree_root;
                let block_number = validity_prover
                    .get_block_number_by_tx_tree_root(tx_tree_root)
                    .await?;
                if let Some(block_number) = block_number {
                    // set block number
                    let mut meta = meta;
                    meta.block_number = Some(block_number);
                    settled.push((meta, transfer_data));
                } else {
                    if meta.timestamp + tx_timeout < chrono::Utc::now().timestamp() as u64 {
                        // timeout
                        log::error!("Transfer {} is timeouted", meta.uuid);
                        rejected.push(meta);
                    } else {
                        // pending
                        log::info!("Transfer {} is pending", meta.uuid);
                        pending.push(meta);
                    }
                }
            }
            Err(e) => {
                log::error!("failed to decrypt transfer data: {}", e);
                rejected.push(meta);
            }
        };
    }

    // sort by block number
    settled.sort_by_key(|(meta, _)| meta.block_number.unwrap());

    Ok(TransferInfo {
        settled,
        pending,
        rejected,
    })
}
