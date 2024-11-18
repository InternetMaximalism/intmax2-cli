use std::sync::Arc;

use async_trait::async_trait;
use ethers::{
    contract::abigen,
    core::k256::ecdsa::SigningKey,
    middleware::SignerMiddleware,
    providers::{Http, Provider},
    signers::Wallet,
    types::{Address, H256},
};
use intmax2_zkp::ethereum_types::{bytes32::Bytes32, u256::U256, u32limb_trait::U32LimbTrait as _};

use super::{
    handlers::handle_contract_call,
    interface::{BlockchainError, ContractInterface},
    utils::{get_address, get_client, get_client_with_signer},
};

abigen!(Liquidity, "abi/Liquidity.json",);

pub async fn get_liquidity_contract(
    rpc_url: &str,
    contract_address: Address,
) -> Result<liquidity::Liquidity<Provider<Http>>, BlockchainError> {
    let client = get_client(rpc_url).await?;
    let contract = Liquidity::new(contract_address, client);
    Ok(contract)
}

pub async fn get_liquidity_contract_with_signer(
    rpc_url: &str,
    chain_id: u64,
    contract_address: Address,
    private_key: H256,
) -> Result<
    liquidity::Liquidity<SignerMiddleware<Provider<Http>, Wallet<SigningKey>>>,
    BlockchainError,
> {
    let client = get_client_with_signer(rpc_url, chain_id, private_key).await?;
    let contract = Liquidity::new(contract_address, Arc::new(client));
    Ok(contract)
}

#[derive(Debug, Clone)]
pub struct LiquidityContract {
    pub rpc_url: String,
    pub chain_id: u64,
    pub contract_address: Address,
}

impl LiquidityContract {
    pub fn new(rpc_url: String, chain_id: u64, contract_address: Address) -> Self {
        Self {
            rpc_url,
            chain_id,
            contract_address,
        }
    }
}

#[async_trait(?Send)]
impl ContractInterface for LiquidityContract {
    async fn deposit(
        &self,
        signer_private_key: H256,
        pubkey_salt_hash: Bytes32,
        token_index: u32,
        amount: U256,
    ) -> Result<(), BlockchainError> {
        if token_index != 0 {
            return Err(BlockchainError::InternalError(
                "Only native token is supported".to_string(),
            ));
        }

        let contract = get_liquidity_contract_with_signer(
            &self.rpc_url,
            self.chain_id,
            self.contract_address,
            signer_private_key,
        )
        .await?;
        let recipient_salt_hash: [u8; 32] = pubkey_salt_hash.to_bytes_be().try_into().unwrap();
        let amount = ethers::types::U256::from_big_endian(&amount.to_bytes_be());
        let mut tx = contract
            .deposit_native_token(recipient_salt_hash)
            .value(amount);
        handle_contract_call(
            &mut tx,
            get_address(self.chain_id, signer_private_key),
            "depositer",
            "deposit_native_token",
        )
        .await?;
        Ok(())
    }
}
