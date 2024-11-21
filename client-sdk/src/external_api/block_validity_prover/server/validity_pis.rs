use intmax2_zkp::{
    circuits::validity::validity_pis::ValidityPublicInputs, ethereum_types::bytes32::Bytes32,
};

use crate::external_api::{
    common::{error::ServerError, response::ServerCommonResponse},
    utils::{
        handler::{handle_response, ResponseType},
        retry::with_retry,
    },
};

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetValidityPisResponse {
    pub validity_public_inputs: ValidityPublicInputs,
    pub senders: Vec<Sender>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sender {
    pub public_key: Bytes32,
    pub is_valid: bool,
}

pub async fn get_validity_pis(
    server_base_url: &str,
    block_number: u32,
) -> Result<Option<ValidityPublicInputs>, ServerError> {
    let url = format!(
        "{}/block-validity-public-inputs?blockNumber={}",
        server_base_url, block_number
    );
    let response = with_retry(|| async { reqwest_wasm::Client::new().get(&url).send().await })
        .await
        .map_err(|e| {
            ServerError::NetworkError(format!(
                "Failed to get validity public inputs: {}",
                e.to_string()
            ))
        })?;
    match handle_response(response).await? {
        ResponseType::Success(response) => {
            let info: ServerCommonResponse<GetValidityPisResponse> =
                response.json().await.map_err(|e| {
                    ServerError::DeserializationError(format!(
                        "Failed to parse validity public inputs response: {}",
                        e.to_string()
                    ))
                })?;
            let validity_public_inputs = info.data.validity_public_inputs;
            Ok(Some(validity_public_inputs))
        }
        ResponseType::NotFound(_) => Ok(None),
        _ => Err(ServerError::UnknownError(
            "Failed to get validity public inputs".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use anyhow::ensure;

    use crate::utils::init_logger::init_logger;

    #[tokio::test]
    async fn test_get_validity_pis() -> anyhow::Result<()> {
        init_logger();
        let server_base_url = "http://localhost:4000/v1/blockvalidity";
        let block_number = 1;
        let result = super::get_validity_pis(server_base_url, block_number).await?;
        ensure!(result.is_some());
        Ok(())
    }
}
