#[cfg(target_arch = "wasm32")]
use gloo_timers::future::sleep;
#[cfg(target_arch = "wasm32")]
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use tokio::time::{sleep, Duration};

/// Wait until the specified timestamp
async fn sleep_until(target: u64) {
    loop {
        let now = chrono::Utc::now().timestamp() as u64;
        if now >= target {
            break;
        }
        // Use appropriate sleep function for WASM and native
        sleep(Duration::from_secs(1)).await;
    }
}

/// Sleep function that works correctly even when PC is in sleep mode
pub async fn sleep_for(seconds: u64) {
    let target = chrono::Utc::now().timestamp() as u64 + seconds;
    sleep_until(target).await;
}
