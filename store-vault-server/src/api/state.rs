use tokio::sync::RwLock;

use super::store_vault_server::StoreVaultServer;

pub struct State {
    pub store_vault_server: RwLock<StoreVaultServer>,
}

impl State {
    pub fn new() -> Self {
        Self {
            store_vault_server: RwLock::new(StoreVaultServer::new()),
        }
    }
}
