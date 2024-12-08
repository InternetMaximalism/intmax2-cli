use actix_cors::Cors;
use actix_web::{middleware::Logger, web::Data, App, HttpServer};
use env_logger::fmt::Formatter;
use log::{LevelFilter, Record};
use std::{
    env,
    fs::File,
    io::{self, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::time::interval;
use validity_prover::{
    api::{api::validity_prover_scope, state::State, validity_prover::ValidityProver},
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
    let validity_prover = ValidityProver::new(&env).await.map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to create validity prover: {}", e),
        )
    })?;
    let inner_state = State::new(validity_prover);
    let state = Data::new(inner_state.clone());

    let is_syncing = Arc::new(AtomicBool::new(false));
    let is_syncing_clone = is_syncing.clone();
    actix_web::rt::spawn(async move {
        let mut interval = interval(Duration::from_secs(env.sync_interval));
        loop {
            interval.tick().await;

            // Skip if previous task is still running
            if is_syncing_clone.load(Ordering::SeqCst) {
                log::warn!("Previous sync task is still running, skipping this interval");
                continue;
            }

            is_syncing_clone.store(true, Ordering::SeqCst);

            match inner_state.sync_task().await {
                Ok(_) => {
                    log::debug!("Sync task completed successfully");
                }
                Err(e) => {
                    log::error!("Error in sync task: {:?}", e);
                }
            }

            // Reset the flag after task completion
            is_syncing_clone.store(false, Ordering::SeqCst);
        }
    });
    HttpServer::new(move || {
        let cors = Cors::permissive();
        App::new()
            .wrap(cors)
            .wrap(Logger::new("Request: %r | Status: %s | Duration: %Ts"))
            .app_data(state.clone())
            .service(health_check)
            .service(validity_prover_scope())
    })
    .bind(format!("0.0.0.0:{}", env.port))?
    .run()
    .await
}
