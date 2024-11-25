use ethers::{
    abi::Detokenize,
    core::k256::ecdsa::SigningKey,
    middleware::SignerMiddleware,
    providers::{Http, Provider},
    signers::Wallet,
    types::{Address, H256},
};

use super::interface::BlockchainError;

pub async fn handle_contract_call<S: ToString, O: Detokenize>(
    tx: &mut ethers::contract::builders::ContractCall<
        SignerMiddleware<Provider<Http>, Wallet<SigningKey>>,
        O,
    >,
    from_address: Address,
    from_name: S,
    tx_name: S,
) -> Result<H256, BlockchainError> {
    set_gas_price(tx)?;
    let result = tx.send().await;
    match result {
        Ok(tx) => {
            let pending_tx = tx;
            log::info!(
                "{} tx hash: {:?}",
                tx_name.to_string(),
                pending_tx.tx_hash()
            );
            let tx_receipt = pending_tx
                .await
                .map_err(|e| {
                    BlockchainError::InternalError(format!("Error awaiting tx receipt: {:?}", e))
                })?
                .unwrap(); // unwrap is safe here
            if tx_receipt.status.unwrap() != 1.into() {
                return Err(BlockchainError::TransactionFailed(format!(
                    "{} failed with tx hash: {:?}",
                    tx_name.to_string(),
                    tx_receipt.transaction_hash
                )));
            }
            return Ok(tx_receipt.transaction_hash);
        }
        Err(e) => {
            let error_message = e.to_string();
            // insufficient balance
            if error_message.contains("-32000") {
                return Err(BlockchainError::InsufficientFunds(format!(
                    "Insufficient funds for {} from {} {:?}",
                    tx_name.to_string(),
                    from_name.to_string(),
                    from_address
                )));
            } else {
                return Err(BlockchainError::InternalError(format!(
                    "Unknown error sending transaction: {:?}",
                    e
                )));
            }
        }
    }
}

fn set_gas_price<O>(
    _tx: &mut ethers::contract::builders::ContractCall<
        SignerMiddleware<Provider<Http>, Wallet<SigningKey>>,
        O,
    >,
) -> Result<(), BlockchainError> {
    // todo: implement gas price setting
    // let result = get_gas_estimation().await?;
    // let inner_tx = tx
    //     .tx
    //     .as_eip1559_mut()
    //     .ok_or(anyhow::anyhow!("EIP-1559 tx expected"))?;
    // *inner_tx = inner_tx
    //     .clone()
    //     .max_priority_fee_per_gas(result.max_priority_fee_per_gas)
    //     .max_fee_per_gas(result.max_fee_per_gas);
    Ok(())
}
