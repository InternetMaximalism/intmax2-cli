use env_logger::Env;
use std::sync::OnceLock;

static LOGGER: OnceLock<()> = OnceLock::new();

pub fn init_logger() {
    LOGGER.get_or_init(|| {
        env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    });
}
