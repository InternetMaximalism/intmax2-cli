use intmax2_core_sdk::external_api::{common::error::ServerError, utils::retry::with_retry};

use super::types::BlockBuilderInfo;

pub struct IndexerApi {
    pub client: reqwest::Client,
    pub base_url: String,
}

impl IndexerApi {
    pub fn new(base_url: &str) -> Self {
        IndexerApi {
            client: reqwest::Client::new(),
            base_url: base_url.to_string(),
        }
    }

    pub async fn get_block_builder_info(&self) -> Result<Vec<BlockBuilderInfo>, ServerError> {
        let url = format!("{}/v1/indexer/builders", self.base_url,);
        let response = with_retry(|| async { self.client.get(&url).send().await })
            .await
            .map_err(|e| {
                ServerError::NetworkError(format!("Failed to get block builder info: {}", e))
            })?;
        if !response.status().is_success() {
            return Err(ServerError::ServerError(format!(
                "Failed to get block builder info: {}",
                response.status()
            )));
        }
        let response = response
            .json::<Vec<BlockBuilderInfo>>()
            .await
            .map_err(|e| {
                ServerError::DeserializationError(format!(
                    "Failed to deserialize block builder info: {}",
                    e
                ))
            })?;
        Ok(response)
    }
}
