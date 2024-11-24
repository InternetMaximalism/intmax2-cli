use std::{io, time::Duration};

use actix_cors::Cors;
use actix_web::{middleware::Logger, web::Data, App, HttpServer};
use ethers::types::Address;
use intmax2_client_sdk::utils::init_logger::init_logger;
use serde::Deserialize;
use tokio::time::interval;
use validity_prover::{
    api::{api::validity_prover_scope, state::State, validity_prover::ValidityProver},
    health_check::health_check,
};

#[derive(Deserialize)]
struct Config {
    port: u16,
    sync_interval: u64,
    rpc_url: String,
    chain_id: u64,
    rollup_contract_address: Address,
    rollup_contract_deployed_block_number: u64,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_logger();
    dotenv::dotenv().ok();
    let config: Config = envy::from_env().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to parse environment variables: {}", e),
        )
    })?;
    let validity_prover = ValidityProver::new(
        &config.rpc_url,
        config.chain_id,
        config.rollup_contract_address,
        config.rollup_contract_deployed_block_number,
    );
    let inner_state = State::new(validity_prover);
    let state = Data::new(inner_state.clone());
    actix_web::rt::spawn(async move {
        let mut interval = interval(Duration::from_secs(config.sync_interval));
        loop {
            interval.tick().await;
            match inner_state.sync_task().await {
                Ok(_) => {}
                Err(e) => log::error!("Error in sync task: {:?}", e),
            }
        }
    });
    HttpServer::new(move || {
        let cors = Cors::permissive();
        App::new()
            .wrap(cors)
            .wrap(Logger::new("Request: %r | Status: %s | Duration: %Ts"))
            .app_data(state.clone())
            .service(health_check)
            .service(validity_prover_scope())
    })
    .bind(format!("0.0.0.0:{}", config.port))?
    .run()
    .await
}
