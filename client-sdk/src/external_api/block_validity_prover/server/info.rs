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
struct GetInfoResponse {
    deposit_index: usize,
    block_number: u32,
}

pub async fn get_info(server_base_url: &str) -> Result<u32, ServerError> {
    let url = format!("{}/info", server_base_url);
    let response = with_retry(|| async { reqwest_wasm::get(&url).await })
        .await
        .map_err(|e| ServerError::NetworkError(format!("Failed to get info: {}", e)))?;
    match handle_response(response).await? {
        ResponseType::Success(response) => {
            let info: ServerCommonResponse<GetInfoResponse> =
                response.json().await.map_err(|e| {
                    ServerError::DeserializationError(format!(
                        "Failed to parse info response: {}",
                        e
                    ))
                })?;
            Ok(info.data.block_number)
        }
        _ => return Err(ServerError::InternalError("Failed to get info".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        external_api::block_validity_prover::server::info::get_info,
        utils::init_logger::init_logger,
    };

    #[tokio::test]
    async fn test_get_info() -> anyhow::Result<()> {
        init_logger();

        let server_base_url = "http://localhost:4000/v1/blockvalidity";
        let block_number = get_info(server_base_url).await?;
        log::info!("block_number: {}", block_number);

        Ok(())
    }
}
