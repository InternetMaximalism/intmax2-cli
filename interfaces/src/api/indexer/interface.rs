use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::api::error::ServerError;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockBuilderInfo {
    pub fee: f64,
    pub speed: u32,
    pub url: String,
}

#[async_trait(?Send)]
pub trait IndexerClientInterface {
    async fn get_block_builder_info(&self) -> Result<Vec<BlockBuilderInfo>, ServerError>;
}
