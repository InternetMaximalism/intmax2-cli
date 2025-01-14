use std::sync::Arc;

use crate::Env;

use super::withdrawal_server::WithdrawalServer;

#[derive(Clone)]
pub struct State {
    pub withdrawal_server: Arc<WithdrawalServer>,
}

impl State {
    pub async fn new(env: &Env) -> anyhow::Result<Self> {
        let withdrawal_server = WithdrawalServer::new(
            &env.database_url,
            env.database_max_connections,
            env.database_timeout,
        )
        .await?;
        Ok(State {
            withdrawal_server: Arc::new(withdrawal_server),
        })
    }
}
