use std::{fs::File, io};

use actix_cors::Cors;
use actix_web::{middleware::Logger, web::Data, App, HttpServer};
use env_logger::fmt::Formatter;
use log::{LevelFilter, Record};
use std::io::Write;
use withdrawal_server::{
    api::{api::withdrawal_server_scope, state::State},
    health_check::health_check,
    Env,
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

    let env = envy::from_env::<Env>()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("env error: {}", e)))?;
    let state = State::new(&env.database_url)
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
