use actix_cors::Cors;
use actix_web::{middleware::Logger, web::Data, App, HttpServer};
use server_common::{
    health_check::{health_check, set_name_and_version},
    logger::init_logger,
};
use std::io::{self};

use validity_prover::{
    api::{coordinator::coordinator_scope, state::State, witness_generator::validity_prover_scope},
    Env,
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    set_name_and_version(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    init_logger().map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    dotenv::dotenv().ok();
    let env: Env = envy::from_env().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to parse environment variables: {}", e),
        )
    })?;
    let state = State::new(&env).await.map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to create validity prover: {}", e),
        )
    })?;

    // Start a job
    state.job();

    let data = Data::new(state.clone());

    HttpServer::new(move || {
        let cors = Cors::permissive();
        App::new()
            .wrap(cors)
            .wrap(Logger::new("Request: %r | Status: %s | Duration: %Ts"))
            .app_data(data.clone())
            .service(health_check)
            .service(validity_prover_scope())
            .service(coordinator_scope())
    })
    .bind(format!("0.0.0.0:{}", env.port))?
    .run()
    .await
}
