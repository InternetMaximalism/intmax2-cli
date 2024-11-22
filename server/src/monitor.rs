use std::time::Duration;
use sysinfo::System;
use tokio::time::interval;

pub async fn monitor_system() {
    let mut interval = interval(Duration::from_secs(30));
    let mut sys = System::new_all();

    loop {
        interval.tick().await;
        sys.refresh_all();

        let total_memory = sys.total_memory() as f64 / (1 << 30) as f64; // GB
        let used_memory = sys.used_memory() as f64 / (1 << 30) as f64; // GB

        let cpu_usage: f32 =
            sys.cpus().iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / sys.cpus().len() as f32;

        log::info!(
            "System Monitor - CPU Usage: {:.1}%, Memory Usage - Total: {:.2} GB, Used: {:.2} GB, Free: {:.2} GB",
            cpu_usage,
            total_memory,
            used_memory,
            total_memory - used_memory
        );
    }
}
