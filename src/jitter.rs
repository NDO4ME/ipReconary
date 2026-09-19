use rand::Rng;
use std::time::Duration;
use tokio::time::sleep;

/// Light jitter for WAF bypass (default)
/// - 10-50ms random delay (minimal impact on scan speed)
pub async fn apply_jitter() {
    let delay = {
        let mut rng = rand::thread_rng();
        rng.gen_range(10..=50)
    };

    sleep(Duration::from_millis(delay)).await;
}
