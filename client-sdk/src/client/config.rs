use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientConfig {
    pub deposit_timeout: u64,
    pub tx_timeout: u64,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            deposit_timeout: 0,
            tx_timeout: 0,
        }
    }
}
