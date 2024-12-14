use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientConfig {
    pub deposit_timeout: u64,
    pub tx_timeout: u64,
    pub block_builder_request_interval: u64,
    pub block_builder_request_limit: u64,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            deposit_timeout: 7200,
            tx_timeout: 60,
            block_builder_request_interval: 5,
            block_builder_request_limit: 10,
        }
    }
}
