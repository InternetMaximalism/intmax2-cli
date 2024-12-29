use ethers::providers::{Http, Middleware, Provider};
use intmax2_client_sdk::external_api::utils::retry::with_retry;
use std::{env, sync::Arc};

pub async fn get_start_block_number() -> anyhow::Result<u64> {
    let start_block_number = std::env::var("START_BLOCK_NUMBER").ok();
    if let Some(start_block_number) = start_block_number {
        let start_block_number: u64 = start_block_number.parse()?;
        println!("Use start block number from env: {}", start_block_number);
        return Ok(start_block_number);
    } else {
        let start_block_number = env::var("LIQUIDITY_CONTRACT_DEPLOYED_BLOCK_NUMBER")
            .map_err(|_| anyhow::anyhow!("LIQUIDITY_CONTRACT_DEPLOYED_BLOCK_NUMBER is not set"))?;
        println!("Use start block number from config: {}", start_block_number);
        return Ok(start_block_number.parse()?);
    }
}

pub fn get_rpc_url() -> anyhow::Result<String> {
    let rpc_url = env::var("L1_RPC_URL").map_err(|_| anyhow::anyhow!("L1_RPC_URL is not set"))?;
    Ok(rpc_url)
}

async fn get_provider() -> anyhow::Result<Provider<Http>> {
    let rpc_url = get_rpc_url()?;
    let provider = Provider::<Http>::try_from(rpc_url)
        .map_err(|_| anyhow::anyhow!("Failed to parse RPC_URL"))?;
    Ok(provider)
}

pub async fn get_latest_block_number() -> anyhow::Result<u64> {
    let provider = get_provider().await?;
    let client = Arc::new(provider);
    let block_number = with_retry(|| async { client.get_block_number().await })
        .await
        .map_err(|_| anyhow::anyhow!("failed to get block number"))?;
    Ok(block_number.as_u64())
}
