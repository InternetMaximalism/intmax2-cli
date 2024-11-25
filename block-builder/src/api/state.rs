use std::{sync::Arc, time::Duration};

use tokio::sync::RwLock;

use crate::Env;

use super::{block_builder::BlockBuilder, error::BlockBuilderError};

#[derive(Debug, Clone)]
pub struct State {
    pub is_shutting_down: Arc<RwLock<bool>>,
    pub block_builder: Arc<RwLock<BlockBuilder>>,
}

impl State {
    pub fn new(block_builder: BlockBuilder) -> Self {
        State {
            is_shutting_down: Arc::new(RwLock::new(false)),
            block_builder: Arc::new(RwLock::new(block_builder)),
        }
    }

    pub async fn job(self, is_registration_block: bool) {
        actix_web::rt::spawn(async move {
            loop {
                if self.is_shutting_down.read().await.clone() {
                    log::info!("Shutting down block builder");
                    break;
                }
                match self.cycle(is_registration_block).await {
                    Ok(_) => {
                        log::info!("Cycle successful");
                    }
                    Err(e) => {
                        log::error!("Error in block builder: {}", e);
                    }
                }
            }
        });
    }

    async fn cycle(&self, is_registration_block: bool) -> Result<(), BlockBuilderError> {
        let env = envy::from_env::<Env>().unwrap();

        self.block_builder
            .write()
            .await
            .start_accepting_txs(is_registration_block)?;

        tokio::time::sleep(Duration::from_secs(env.accepting_tx_interval)).await;

        self.block_builder
            .write()
            .await
            .construct_block(is_registration_block)?;

        tokio::time::sleep(Duration::from_secs(env.proposing_block_interval)).await;

        self.block_builder
            .write()
            .await
            .post_block(is_registration_block)
            .await?;

        Ok(())
    }
}
