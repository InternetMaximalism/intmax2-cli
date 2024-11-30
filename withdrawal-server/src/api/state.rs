use std::sync::Arc;

use super::withdrawal_server::WithdrawalServer;

#[derive(Clone)]
pub struct State {
    pub withdrawl_server: Arc<WithdrawalServer>,
}

impl State {
    pub async fn new(database_url: &str) -> anyhow::Result<Self> {
        let withdrawal_server = WithdrawalServer::new(database_url).await?;
        Ok(State {
            withdrawl_server: Arc::new(withdrawal_server),
        })
    }
}
