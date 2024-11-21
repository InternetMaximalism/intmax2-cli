use intmax2_zkp::ethereum_types::bytes32::Bytes32;
use serde::{Deserialize, Serialize};

use crate::external_api::{
    common::{error::ServerError, response::ServerCommonResponse},
    utils::{
        encode::decode_base64,
        handler::{handle_response, ResponseType},
        retry::with_retry,
    },
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetUserDataResponse {
    pub user: Bytes32,
    pub encrypted_user_data: String,
}

pub async fn get_user_data(
    server_base_url: &str,
    pubkey: Bytes32,
) -> Result<Option<Vec<u8>>, ServerError> {
    let url = format!("{}/user-data/{}", server_base_url, pubkey);
    let response = with_retry(|| async { reqwest_wasm::Client::new().get(&url).send().await })
        .await
        .map_err(|e| {
            ServerError::NetworkError(format!("Failed to get user data from server: {}", e))
        })?;
    match handle_response(response).await? {
        ResponseType::Success(response) => {
            let response = response
                .json::<ServerCommonResponse<GetUserDataResponse>>()
                .await
                .map_err(|e| {
                    ServerError::DeserializationError(format!(
                        "Failed to deserialize user data response: {}",
                        e
                    ))
                })?;
            if !response.success {
                return Err(ServerError::InvalidResponse(
                    "Failed to get user data".to_string(),
                ));
            }
            if response.data.user != pubkey {
                return Err(ServerError::InvalidResponse(
                    "User data does not match the requested user".to_string(),
                ));
            }
            let data = decode_base64(&response.data.encrypted_user_data).map_err(|e| {
                ServerError::DeserializationError(format!(
                    "Failed to decode user data: {}",
                    e.to_string()
                ))
            })?;
            Ok(Some(data))
        }
        ResponseType::NotFound(error) => {
            log::warn!("Failed to get user data: {}", error.message);
            Ok(None)
        }
        ResponseType::ServerError(error) => {
            log::error!("Failed to get user data: {}", error.message);
            Err(ServerError::ServerError(error.message))
        }
        ResponseType::UnknownError(error) => {
            log::error!("Failed to get user data: {}", error);
            Err(ServerError::UnknownError(error))
        }
    }
}

#[cfg(test)]
mod tests {}
