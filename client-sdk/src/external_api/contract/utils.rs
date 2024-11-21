use std::sync::Arc;

use ethers::{
    core::k256::{ecdsa::SigningKey, SecretKey},
    middleware::SignerMiddleware,
    providers::{Http, Provider},
    signers::{Signer as _, Wallet},
    types::{Address, H256},
};

use super::interface::BlockchainError;

async fn get_provider(rpc_url: &str) -> Result<Provider<Http>, BlockchainError> {
    let provider = Provider::<Http>::try_from(rpc_url)
        .map_err(|_| BlockchainError::InternalError("Failed to parse RPC_URL".to_string()))?;
    Ok(provider)
}

pub async fn get_client(rpc_url: &str) -> Result<Arc<Provider<Http>>, BlockchainError> {
    Ok(Arc::new(get_provider(rpc_url).await?))
}

pub fn get_wallet(chain_id: u64, private_key: H256) -> Wallet<SigningKey> {
    let key = SecretKey::from_bytes(private_key.as_bytes().into()).unwrap();
    Wallet::from(key).with_chain_id(chain_id)
}

pub fn get_address(chain_id: u64, private_key: H256) -> Address {
    get_wallet(chain_id, private_key).address()
}

pub async fn get_client_with_signer(
    rpc_url: &str,
    chain_id: u64,
    private_key: H256,
) -> Result<SignerMiddleware<Provider<Http>, Wallet<SigningKey>>, BlockchainError> {
    let provider = get_provider(rpc_url).await?;
    let wallet = get_wallet(chain_id, private_key);
    let client = SignerMiddleware::new(provider, wallet);
    Ok(client)
}
