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

#[cfg(test)]
mod tests {
    use mockito::Server;

    use super::*;
    #[tokio::test]
    async fn test_get_block_builder_info() {
        let mut server = Server::new();

        let test_data = vec![BlockBuilderInfo {
            fee: 0.001,
            speed: 100,
            url: "http://builder1.example.com".to_string(),
        }];

        let mock = server
            .mock("GET", "/v1/indexer/builders")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&test_data).unwrap())
            .create();

        let client = IndexerApi::new(&server.url());

        let result = client.get_block_builder_info().await;

        assert!(result.is_ok());
        let builders = result.unwrap();
        assert_eq!(builders.len(), 1);
        assert_eq!(builders[0].fee, 0.001);
        assert_eq!(builders[0].speed, 100);
        assert_eq!(builders[0].url, "http://builder1.example.com");

        mock.assert();
    }

    #[tokio::test]
    async fn test_get_block_builder_info_error() {
        let mut server = Server::new();

        let mock = server
            .mock("GET", "/v1/indexer/builders")
            .with_status(500)
            .with_body("Internal Server Error")
            .create();

        let client = IndexerApi::new(&server.url());

        let result = client.get_block_builder_info().await;

        assert!(result.is_err());
        match result {
            Err(ServerError::ServerError(msg)) => {
                assert!(msg.contains("Failed to get block builder info"));
                assert!(msg.contains("500"));
            }
            _ => panic!("Expected ServerError::ServerError"),
        }

        mock.assert();
    }
}
