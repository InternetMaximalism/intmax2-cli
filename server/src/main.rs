// use actix_cors::Cors;
// use std::io;

// use actix_web::{middleware::Logger, web::Data, App, HttpServer};
// use api::{
//     balance_prover::api::balance_prover_scope, block_builder::api::block_builder_scope,
//     block_validity_prover::api::block_validity_prover_scope, contract::api::contract_scope,
//     health_check::health_check, reset::reset, state::State,
//     store_vault_server::api::store_vault_server_scope,
//     withdrawal_aggregator::api::withdrawal_aggregator_scope,
// };
// use log::init_logger;
// use monitor::monitor_system;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // init_logger()?;

    // tokio::spawn(monitor_system());

    // let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    // let state = State::new().map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    // let state = Data::new(state);
    // HttpServer::new(move || {
    //     let cors = Cors::permissive();
    //     App::new()
    //         .wrap(cors)
    //         .wrap(Logger::new("Request: %r | Status: %s | Duration: %Ts"))
    //         .wrap(Logger::default())
    //         .app_data(state.clone())
    //         .service(reset)
    //         .service(health_check)
    //         .service(balance_prover_scope())
    //         .service(block_builder_scope())
    //         .service(block_validity_prover_scope())
    //         .service(contract_scope())
    //         .service(store_vault_server_scope())
    //         .service(withdrawal_aggregator_scope())
    // })
    // .bind(format!("0.0.0.0:{}", port))?
    // .run()
    // .await

    Ok(())
}
