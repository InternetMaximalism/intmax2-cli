use std::str::FromStr;

use anyhow::{bail, ensure};
use serde::{Deserialize, Serialize};

use intmax2_zkp::{
    common::{
        deposit::{get_pubkey_salt_hash, Deposit},
        salt::Salt,
        signature::key_set::KeySet,
    },
    ethereum_types::{address::Address, bytes32::Bytes32, u256::U256},
    utils::leafable::Leafable,
};

use super::encryption::{decrypt, encrypt};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepositData {
    pub deposit_salt: Salt,
    pub pubkey_salt_hash: Bytes32, // The poseidon hash of the pubkey and salt, to hide the pubkey
    pub amount: U256,              // The amount of the token, which is the amount of the deposit

    // token info
    pub token_type: TokenType,
    pub token_address: Address,
    pub token_id: U256,

    pub token_index: Option<u32>, // The index of the token in the contract
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TokenType {
    NATIVE = 0,
    ERC20 = 1,
    ERC721 = 2,
    ERC1155 = 3,
}

impl FromStr for TokenType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "NATIVE" => Ok(Self::NATIVE),
            "ERC20" => Ok(Self::ERC20),
            "ERC721" => Ok(Self::ERC721),
            "ERC1155" => Ok(Self::ERC1155),
            _ => bail!("invalid token type"),
        }
    }
}

impl ToString for TokenType {
    fn to_string(&self) -> String {
        match self {
            Self::NATIVE => "NATIVE".to_string(),
            Self::ERC20 => "ERC20".to_string(),
            Self::ERC721 => "ERC721".to_string(),
            Self::ERC1155 => "ERC1155".to_string(),
        }
    }
}

impl TryFrom<u8> for TokenType {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::NATIVE),
            1 => Ok(Self::ERC20),
            2 => Ok(Self::ERC721),
            3 => Ok(Self::ERC1155),
            _ => Err("invalid token type".to_string()),
        }
    }
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
            self.pubkey_salt_hash == get_pubkey_salt_hash(key.pubkey, self.deposit_salt),
            "invalid pubkey_salt_hash"
        );
        Ok(())
    }

    pub fn set_token_index(&mut self, token_index: u32) {
        self.token_index = Some(token_index);
    }

    pub fn deposit(&self) -> Option<Deposit> {
        if let Some(token_index) = self.token_index {
            Some(Deposit {
                pubkey_salt_hash: self.pubkey_salt_hash,
                token_index,
                amount: self.amount,
            })
        } else {
            None
        }
    }

    pub fn deposit_hash(&self) -> Option<Bytes32> {
        if let Some(deposit) = self.deposit() {
            Some(deposit.hash())
        } else {
            None
        }
    }
}
