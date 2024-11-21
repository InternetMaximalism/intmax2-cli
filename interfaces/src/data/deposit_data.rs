use anyhow::ensure;
use serde::{Deserialize, Serialize};

use intmax2_zkp::{
    common::{
        deposit::{get_pubkey_salt_hash, Deposit},
        salt::Salt,
        signature::key_set::KeySet,
    },
    ethereum_types::{bytes32::Bytes32, u256::U256},
    utils::leafable::Leafable,
};

use super::encryption::{decrypt, encrypt};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepositData {
    pub deposit_salt: Salt,
    pub deposit: Deposit,
}

impl DepositData {
    fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        let data = bincode::deserialize(bytes)?;
        Ok(data)
    }

    pub fn encrypt(&self, pubkey: U256) -> Vec<u8> {
        encrypt(pubkey, &self.to_bytes())
    }

    pub fn decrypt(bytes: &[u8], key: KeySet) -> anyhow::Result<Self> {
        let data = decrypt(key, bytes)?;
        let data = Self::from_bytes(&data)?;
        data.validate(key)?;
        Ok(data)
    }

    fn validate(&self, key: KeySet) -> anyhow::Result<()> {
        ensure!(
            self.deposit.pubkey_salt_hash == get_pubkey_salt_hash(key.pubkey, self.deposit_salt),
            "invalid pubkey_salt_hash"
        );
        Ok(())
    }

    pub fn deposit_hash(&self) -> Bytes32 {
        self.deposit.hash()
    }
}