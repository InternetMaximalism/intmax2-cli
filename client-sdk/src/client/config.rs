use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientConfig {
    pub deposit_timeout: u64,
    pub tx_timeout: u64,
    pub max_tx_request_retries: u64,
    pub tx_request_retry_interval: u64,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            deposit_timeout: 300,
            tx_timeout: 300,
            max_tx_request_retries: 1,
            tx_request_retry_interval: 5,
        }
    }
}
