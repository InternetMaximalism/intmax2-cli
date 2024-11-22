use chrono::Local;
use env_logger::{Builder, Target};
use log::LevelFilter;
use std::fs::OpenOptions;
use std::io::Write;

pub fn init_logger() -> std::io::Result<()> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("application.log")?;

    Builder::new()
        .target(Target::Pipe(Box::new(file)))
        .target(Target::Stdout)
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - {}: {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                record.args()
            )
        })
        .filter(None, LevelFilter::Info)
        .init();

    Ok(())
}
