use plonky2::{
    field::extension::Extendable, hash::hash_types::RichField, plonk::config::GenericConfig,
};
use serde::{Deserialize, Serialize};

use intmax2_zkp::{
    common::{signature::key_set::KeySet, witness::spent_witness::SpentWitness},
    ethereum_types::u256::U256,
};

use super::{
    common_tx_data::CommonTxData,
    encryption::{decrypt, encrypt},
};

// tx data for sender
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(bound = "")]
pub struct TxData<F, C, const D: usize>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    pub common: CommonTxData<F, C, D>,
    pub spent_witness: SpentWitness, // to update sender's private state
}

impl<F, C, const D: usize> TxData<F, C, D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
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

    pub fn validate(&self, _key: KeySet) -> anyhow::Result<()> {
        self.common.validate()?;
        Ok(())
    }
}
