use std::io;

use actix_cors::Cors;
use actix_web::{middleware::Logger, web::Data, App, HttpServer};
use block_builder::{
    api::{api::block_builder_scope, block_builder::BlockBuilder, state::State},
    health_check::health_check,
    Env,
};
use intmax2_client_sdk::utils::init_logger::init_logger;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_logger();
    dotenv::dotenv().ok();

    let env = envy::from_env::<Env>()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("env error: {}", e)))?;

    let eth_allowance_for_block = ethers::utils::parse_ether(env.eth_allowance_for_block).unwrap();
    let block_builder = BlockBuilder::new(
        &env.l2_rpc_url,
        env.l2_chain_id,
        env.rollup_contract_address,
        env.rollup_contract_deployed_block_number,
        env.block_builder_private_key,
        eth_allowance_for_block.into(),
        &env.validity_prover_base_url,
    );
    let state = State::new(block_builder);

    // Start the block builder job
    let state_for_registration_cycle = state.clone();
    state_for_registration_cycle.job(true).await;
    let state_for_non_registration_cycle = state.clone();
    state_for_non_registration_cycle.job(false).await;

    let state = Data::new(state);
    HttpServer::new(move || {
        let cors = Cors::permissive();
        App::new()
            .wrap(cors)
            .wrap(Logger::new("Request: %r | Status: %s | Duration: %Ts"))
            .app_data(state.clone())
            .service(health_check)
            .service(block_builder_scope())
    })
    .bind(format!("0.0.0.0:{}", env.port))?
    .run()
    .await
}
