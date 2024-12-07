use actix_cors::Cors;
use actix_web::{middleware::Logger, web::Data, App, HttpServer};
use env_logger::fmt::Formatter;
use log::{LevelFilter, Record};
use std::{fs::File, io::Write};
use store_vault_server::{
    api::{api::store_vault_server_scope, state::State},
    health_check::health_check,
};

fn init_file_logger() {
    let log_file = File::create("log.txt").expect("Unable to create log file");
    let log_file = std::sync::Mutex::new(log_file);

    env_logger::Builder::new()
        .format(move |buf: &mut Formatter, record: &Record| {
            let mut log_file = log_file.lock().unwrap();
            writeln!(buf, "{}: {}", record.level(), record.args())?;
            writeln!(log_file, "{}: {}", record.level(), record.args())
        })
        .filter(None, LevelFilter::Info)
        .init();
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_file_logger();

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
