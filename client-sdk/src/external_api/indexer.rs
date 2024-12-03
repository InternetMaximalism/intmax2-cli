use async_trait::async_trait;
use intmax2_interfaces::api::{
    error::ServerError,
    indexer::interface::{BlockBuilderInfo, IndexerClientInterface},
};

use super::utils::query::get_request;

#[derive(Debug, Clone)]
pub struct IndexerClient {
    base_url: String,
}

impl IndexerClient {
    pub fn new(base_url: &str) -> Self {
        IndexerClient {
            base_url: base_url.to_string(),
        }
    }
}

#[async_trait(?Send)]
impl IndexerClientInterface for IndexerClient {
    async fn get_block_builder_info(&self) -> Result<Vec<BlockBuilderInfo>, ServerError> {
        let response: Vec<BlockBuilderInfo> =
            get_request::<(), _>(&self.base_url, "/v1/indexer/builders", None, None).await?;
        Ok(response)
    }
}
