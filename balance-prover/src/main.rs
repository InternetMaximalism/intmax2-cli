use std::io;

use actix_cors::Cors;
use actix_web::{middleware::Logger, web::Data, App, HttpServer};
use balance_prover::{
    api::{api::balance_prover_scope, balance_prover::BalanceProver},
    health_check::health_check,
};
use intmax2_client_sdk::utils::init_logger::init_logger;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_logger();

    dotenv::dotenv().ok();

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let state = BalanceProver::new().map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let state = Data::new(state);
    HttpServer::new(move || {
        let cors = Cors::permissive();
        App::new()
            .wrap(cors)
            .wrap(Logger::new("Request: %r | Status: %s | Duration: %Ts"))
            .app_data(state.clone())
            .service(health_check)
            .service(balance_prover_scope())
    })
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await
}
