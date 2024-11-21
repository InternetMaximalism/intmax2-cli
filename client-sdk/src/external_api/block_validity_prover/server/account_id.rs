use intmax2_zkp::ethereum_types::bytes32::Bytes32;
use serde::{Deserialize, Serialize};

use crate::external_api::{
    common::{error::ServerError, response::ServerCommonResponse},
    utils::{
        handler::{handle_response, ResponseType},
        retry::with_retry,
    },
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetAccountIdResponse {
    is_registered: bool,
    account_id: u64,
}

pub async fn get_account_id(
    server_base_url: &str,
    pubkey: Bytes32,
) -> Result<Option<u64>, ServerError> {
    let url = format!("{}/account/{}", server_base_url, pubkey);
    let response = with_retry(|| async { reqwest_wasm::get(&url).await })
        .await
        .map_err(|e| ServerError::NetworkError(format!("Failed to get account id: {}", e)))?;
    match handle_response(response).await? {
        ResponseType::Success(response) => {
            let info: ServerCommonResponse<GetAccountIdResponse> =
                response.json().await.map_err(|e| {
                    ServerError::DeserializationError(format!(
                        "Failed to parse account id response: {}",
                        e
                    ))
                })?;
            if info.data.is_registered {
                Ok(Some(info.data.account_id))
            } else {
                Ok(None)
            }
        }
        _ => {
            return Err(ServerError::InternalError(
                "Failed to get account id".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use intmax2_zkp::ethereum_types::bytes32::Bytes32;

    use crate::{
        external_api::block_validity_prover::server::account_id::get_account_id,
        utils::init_logger::init_logger,
    };

    #[tokio::test]
    async fn test_get_account_id() -> anyhow::Result<()> {
        init_logger();

        let server_base_url = "http://localhost:4000/v1/blockvalidity";
        let pubkey = Bytes32::default();
        let account_id = get_account_id(server_base_url, pubkey).await?;
        log::info!("account_id: {:?}", account_id);

        Ok(())
    }
}
