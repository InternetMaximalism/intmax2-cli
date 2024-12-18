use std::{env, fs::File, io};

use actix_cors::Cors;
use actix_web::{middleware::Logger, web::Data, App, HttpServer};
use env_logger::fmt::Formatter;
use log::{LevelFilter, Record};
use server_common::logger::init_logger;
use std::io::Write;
use withdrawal_server::{
    api::{api::withdrawal_server_scope, state::State},
    health_check::health_check,
    Env,
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_logger().map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    dotenv::dotenv().ok();

    let env = envy::from_env::<Env>()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("env error: {}", e)))?;
    let state = State::new(&env)
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("state error: {}", e)))?;
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
