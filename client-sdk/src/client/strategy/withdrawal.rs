use intmax2_interfaces::{
    api::{
        store_vault_server::interface::{DataType, StoreVaultClientInterface},
        validity_prover::interface::ValidityProverClientInterface,
    },
    data::{meta_data::MetaData, transfer_data::TransferData},
};
use plonky2::{
    field::{extension::Extendable, goldilocks_field::GoldilocksField},
    hash::hash_types::RichField,
    plonk::config::{GenericConfig, PoseidonGoldilocksConfig},
};

use intmax2_zkp::common::signature::key_set::KeySet;

use super::error::StrategyError;

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Debug, Clone)]
pub struct WithdrawalInfo<F, C, const D: usize>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    pub settled: Vec<(MetaData, TransferData<F, C, D>)>,
    pub pending: Vec<(MetaData, TransferData<F, C, D>)>,
    pub timeout: Vec<(MetaData, TransferData<F, C, D>)>,
}

pub async fn fetch_withdrawal_info<
    S: StoreVaultClientInterface,
    V: ValidityProverClientInterface,
>(
    store_vault_server: &S,
    validity_prover: &V,
    key: KeySet,
    withdrawal_lpt: u64,
    processed_withdrawal_uuids: &[String],
    tx_timeout: u64,
) -> Result<WithdrawalInfo<F, C, D>, StrategyError> {
    let mut settled = Vec::new();
    let mut pending = Vec::new();
    let mut timeout = Vec::new();

    let encrypted_data = store_vault_server
        .get_data_all_after(DataType::Withdrawal, key.pubkey, withdrawal_lpt)
        .await?;
    for (meta, encrypted_data) in encrypted_data {
        if processed_withdrawal_uuids.contains(&meta.uuid) {
            log::info!("Withdrawal {} is already processed", meta.uuid);
            continue;
        }
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
                        log::error!("Withdrawal {} is timeout", meta.uuid);
                        timeout.push((meta, transfer_data));
                    } else {
                        // pending
                        log::info!("Withdrawal {} is pending", meta.uuid);
                        pending.push((meta, transfer_data));
                    }
                }
            }
            Err(e) => {
                log::error!("failed to decrypt withdrawal data: {}", e);
                // ignore this withdrawal
            }
        }
    }

    Ok(WithdrawalInfo {
        settled,
        pending,
        timeout,
    })
}
