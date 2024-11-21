use std::time::Duration;

use gloo_timers::future::sleep;

async fn sleep_until(target: u64) {
    loop {
        let now = chrono::Utc::now().timestamp() as u64;
        if now >= target {
            break;
        }
        sleep(Duration::from_secs(1)).await;
    }
}

// sleep avoiding hanging when PC is in sleep mode
pub async fn sleep_for(seconds: u64) {
    let target = chrono::Utc::now().timestamp() as u64 + seconds;
    sleep_until(target).await;
}
