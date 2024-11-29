use actix_cors::Cors;
use actix_web::{middleware::Logger, web::Data, App, HttpServer};
use intmax2_client_sdk::utils::init_logger::init_logger;
use store_vault_server::{
    api::{api::store_vault_server_scope, state::State},
    health_check::health_check,
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_logger();

    dotenv::dotenv().ok();

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let state = Data::new(State::new(&database_url).await.unwrap());
    HttpServer::new(move || {
        let cors = Cors::permissive();
        App::new()
            .wrap(cors)
            .wrap(Logger::new("Request: %r | Status: %s | Duration: %Ts"))
            .app_data(state.clone())
            .service(health_check)
            .service(store_vault_server_scope())
    })
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await
}
