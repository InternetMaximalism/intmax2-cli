use std::sync::Arc;

use ethers::{
    contract::abigen,
    core::k256::ecdsa::SigningKey,
    middleware::SignerMiddleware,
    providers::{Http, Provider},
    signers::Wallet,
    types::{Address, H256},
};
use intmax2_zkp::ethereum_types::{bytes32::Bytes32, u256::U256, u32limb_trait::U32LimbTrait as _};

use crate::external_api::utils::retry::with_retry;

use super::{
    handlers::handle_contract_call,
    interface::{BlockchainError, ContractWithdrawal},
    utils::{get_address, get_client, get_client_with_signer},
};

abigen!(Liquidity, "abi/Liquidity.json",);

#[derive(Debug, Clone, Copy)]
pub enum TokenType {
    Native = 0,
    ERC20 = 1,
}

#[derive(Debug, Clone)]
pub struct LiquidityContract {
    pub rpc_url: String,
    pub chain_id: u64,
    pub address: Address,
}

impl LiquidityContract {
    pub fn new(rpc_url: &str, chain_id: u64, address: Address) -> Self {
        Self {
            rpc_url: rpc_url.to_string(),
            chain_id,
            address,
        }
    }

    pub async fn get_contract(
        &self,
    ) -> Result<liquidity::Liquidity<Provider<Http>>, BlockchainError> {
        let client = get_client(&self.rpc_url).await?;
        let contract = Liquidity::new(self.address, client);
        Ok(contract)
    }

    async fn get_contract_with_signer(
        &self,
        private_key: H256,
    ) -> Result<
        liquidity::Liquidity<SignerMiddleware<Provider<Http>, Wallet<SigningKey>>>,
        BlockchainError,
    > {
        let client = get_client_with_signer(&self.rpc_url, self.chain_id, private_key).await?;
        let contract = Liquidity::new(self.address, Arc::new(client));
        Ok(contract)
    }

    async fn get_token_index(
        &self,
        token_type: TokenType,
        token_address: Address,
        token_id: U256,
    ) -> Result<u32, BlockchainError> {
        let contract = self.get_contract().await?;
        let token_id = ethers::types::U256::from_big_endian(&token_id.to_bytes_be());
        let (_is_native, token_index) = with_retry(|| async {
            contract
                .get_token_index(token_type as u8, token_address, token_id)
                .call()
                .await
        })
        .await
        .map_err(|e| {
            BlockchainError::NetworkError(format!("Error getting token index: {:?}", e))
        })?;
        Ok(token_index)
    }

    pub async fn deposit_native(
        &self,
        signer_private_key: H256,
        pubkey_salt_hash: Bytes32,
        amount: U256,
    ) -> Result<(), BlockchainError> {
        let contract = self.get_contract_with_signer(signer_private_key).await?;
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

    pub async fn deposit_erc20(
        &self,
        signer_private_key: H256,
        pubkey_salt_hash: Bytes32,
        amount: U256,
        token_address: Address,
    ) -> Result<u32, BlockchainError> {
        let contract = self.get_contract_with_signer(signer_private_key).await?;
        let recipient_salt_hash: [u8; 32] = pubkey_salt_hash.to_bytes_be().try_into().unwrap();
        let amount = ethers::types::U256::from_big_endian(&amount.to_bytes_be());
        let mut tx = contract.deposit_erc20(token_address, recipient_salt_hash, amount);
        handle_contract_call(
            &mut tx,
            get_address(self.chain_id, signer_private_key),
            "depositer",
            "deposit_erc20_token",
        )
        .await?;
        let token_index = self
            .get_token_index(TokenType::ERC20, token_address, 0.into())
            .await?;
        Ok(token_index)
    }

    pub async fn claim_withdrawals(
        &self,
        signer_private_key: H256,
        withdrawals: &[ContractWithdrawal],
    ) -> Result<(), BlockchainError> {
        let withdrawals = withdrawals
            .iter()
            .map(|w| {
                let recipient = ethers::types::Address::from_slice(&w.recipient.to_bytes_be());
                let token_index = w.token_index;
                let amount = ethers::types::U256::from_big_endian(&w.amount.to_bytes_be());
                let id = ethers::types::U256::from(w.id);
                Withdrawal {
                    recipient,
                    token_index,
                    amount,
                    id,
                }
            })
            .collect::<Vec<_>>();
        let contract = self.get_contract_with_signer(signer_private_key).await?;
        let mut tx = contract.claim_withdrawals(withdrawals);
        handle_contract_call(
            &mut tx,
            get_address(self.chain_id, signer_private_key),
            "withdrawer",
            "claim_withdrawals",
        )
        .await?;
        Ok(())
    }
}
