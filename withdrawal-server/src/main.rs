use std::io;

use actix_cors::Cors;
use actix_web::{middleware::Logger, web::Data, App, HttpServer};
use block_builder::{
    api::{api::block_builder_scope, block_builder::BlockBuilder, state::State},
    health_check::health_check,
    Env,
};
use intmax2_client_sdk::utils::init_logger::init_logger;
use tokio::time::sleep;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_logger();
    let port = 
    let state = State;
    let state = Data::new(state);
    HttpServer::new(move || {
        let cors = Cors::permissive();
        App::new()
            .wrap(cors)
            .wrap(Logger::new("Request: %r | Status: %s | Duration: %Ts"))
            .app_data(state.clone())
            .service(health_check)
            .service(withdrawal_server_scope())
    })
    .bind(format!("0.0.0.0:{}", env.port))?
    .run()
    .await
}
