use actix_cors::Cors;
use actix_web::{middleware::Logger, web::Data, App, HttpServer};
use balance_prover::{
    api::{api::balance_prover_scope, balance_prover::BalanceProver},
    health_check::health_check,
    Env,
};
use env_logger::fmt::Formatter;
use log::{LevelFilter, Record};
use std::{
    env,
    fs::File,
    io::{self, Write},
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
    .bind(format!("0.0.0.0:{}", env.port))?
    .run()
    .await
}
