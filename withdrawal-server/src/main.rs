use std::env;

use actix_cors::Cors;
use actix_web::{middleware::Logger, web::Data, App, HttpServer};
use intmax2_client_sdk::utils::init_logger::init_logger;
use withdrawal_server::{
    api::{api::withdrawal_server_scope, state::State},
    health_check::health_check,
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_logger();

    dotenv::dotenv().ok();
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());

    let state = State {};
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
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await
}
