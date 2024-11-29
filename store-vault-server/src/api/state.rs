use anyhow::Result;
use tokio::sync::RwLock;

use super::store_vault_server::StoreVaultServer;

pub struct State {
    pub store_vault_server: RwLock<StoreVaultServer>,
}

impl State {
    pub async fn new(database_url: &str) -> Result<Self> {
        Ok(Self {
            store_vault_server: RwLock::new(StoreVaultServer::new(database_url).await?),
        })
    }
}
