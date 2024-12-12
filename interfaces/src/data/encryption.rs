use intmax2_zkp::{common::signature::key_set::KeySet, ethereum_types::u256::U256};

pub(super) fn encrypt(_pubkey: U256, data: &[u8]) -> Vec<u8> {
    // todo: add encryption
    data.to_vec()
}

pub(super) fn decrypt(_key: KeySet, encrypted_data: &[u8]) -> anyhow::Result<Vec<u8>> {
    // todo: add decryption
    Ok(encrypted_data.to_vec())
}
