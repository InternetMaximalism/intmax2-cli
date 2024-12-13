use actix_cors::Cors;
use actix_web::{
    middleware::Logger,
    web::{Data, JsonConfig},
    App, HttpServer,
};
use env_logger::fmt::Formatter;
use log::{LevelFilter, Record};
use std::{
    env,
    fs::File,
    io::{self, Write},
};
use store_vault_server::{
    api::{api::store_vault_server_scope, state::State, store_vault_server::StoreVaultServer},
    health_check::health_check,
    Env,
};

fn init_file_logger() {
    let mut builder = env_logger::Builder::new();

    if env::var("LOG_TO_FILE").unwrap_or_default() == "1" {
        let log_file = File::create("log.txt").expect("Unable to create log file");
        let log_file = std::sync::Mutex::new(log_file);
        builder.format(move |buf: &mut Formatter, record: &Record| {
            writeln!(buf, "{}: {}", record.level(), record.args())?;
            if let Ok(mut file) = log_file.lock() {
                writeln!(file, "{}: {}", record.level(), record.args())?;
            }
            Ok(())
        });
    } else {
        builder.format(|buf, record| writeln!(buf, "{}: {}", record.level(), record.args()));
    }
    builder.filter(None, LevelFilter::Info).init();
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_file_logger();

    dotenv::dotenv().ok();

    let env: Env = envy::from_env().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to parse environment variables: {}", e),
        )
    })?;
    let store_vault_server = StoreVaultServer::new(&env).await.map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to initialize store_vault_server: {}", e),
        )
    })?;
    let state = Data::new(State::new(store_vault_server));
    HttpServer::new(move || {
        let cors = Cors::permissive();
        App::new()
            .wrap(cors)
            .wrap(Logger::new("Request: %r | Status: %s | Duration: %Ts"))
            .app_data(JsonConfig::default().limit(35_000_000))
            .app_data(state.clone())
            .service(health_check)
            .service(store_vault_server_scope())
    })
    .bind(format!("0.0.0.0:{}", env.port))?
    .run()
    .await
}
