use thiserror::Error;
use tracing::level_filters::LevelFilter;
use tracing_appender::rolling::{InitError, RollingFileAppender, Rotation};
use tracing_subscriber::{
    fmt,
    layer::SubscriberExt as _,
    util::{SubscriberInitExt as _, TryInitError},
    Layer as _,
};

use crate::health_check::load_name_and_version;

const LOG_DIR: &str = "logs";
const MAX_LOG_FILES: usize = 14;

#[derive(Error, Debug)]
pub enum InitLoggerError {
    #[error("Failed to initialize logger: {0}")]
    SetGlobalSubscriberError(#[from] tracing::subscriber::SetGlobalDefaultError),

    #[error("Failed to build file appender: {0}")]
    FailedToBuildFileAppender(#[from] InitError),

    #[error("Failed to initialize logger: {0}")]
    TryInitError(#[from] TryInitError),
}

pub fn init_logger() -> Result<(), InitLoggerError> {
    // Get package info for log file naming
    let (name, _version) = load_name_and_version();
    let log_file_name = format!("{}.log", name);

    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .max_log_files(MAX_LOG_FILES)
        .filename_prefix(&log_file_name)
        .build(LOG_DIR)?;

    let subscriber = tracing_subscriber::registry()
        .with(
            // Log to stdout
            fmt::Layer::new()
                .with_target(false)
                .pretty()
                .with_filter(LevelFilter::INFO),
        )
        .with(
            // Log to file
            fmt::Layer::new()
                .with_target(false)
                .with_ansi(false)
                .pretty()
                .with_writer(file_appender)
                .with_filter(LevelFilter::INFO),
        );

    // Initialize the global subscriber
    subscriber.try_init()?;
    Ok(())
}
