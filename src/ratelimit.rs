use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;

/// Token Bucket Rate Limiter
pub struct RateLimiter {
    rate: u64,           // requests per second
    max_tokens: u64,     // burst capacity (2x rate)
    tokens: AtomicU64,
    last_update: Mutex<Instant>,
}

impl RateLimiter {
    pub fn new(rate: u64) -> Arc<Self> {
        let max_tokens = if rate == 0 { u64::MAX } else { rate * 2 };
        let initial_tokens = if rate == 0 { u64::MAX } else { max_tokens };

        Arc::new(Self {
            rate,
            max_tokens,
            tokens: AtomicU64::new(initial_tokens),
            last_update: Mutex::new(Instant::now()),
        })
    }

    /// Acquire a token, waiting if necessary
    pub async fn acquire(&self) {
        // rate = 0 means unlimited
        if self.rate == 0 {
            return;
        }

        loop {
            self.refill().await;

            let current = self.tokens.load(Ordering::Relaxed);
            if current > 0 {
                if self
                    .tokens
                    .compare_exchange(current, current - 1, Ordering::Relaxed, Ordering::Relaxed)
                    .is_ok()
                {
                    return;
                }
            } else {
                // Wait a bit before trying again
                sleep(Duration::from_millis(10)).await;
            }
        }
    }

    async fn refill(&self) {
        let mut last = self.last_update.lock().await;
        let now = Instant::now();
        let elapsed = now.duration_since(*last);

        // Calculate tokens to add based on elapsed time
        let tokens_to_add = (elapsed.as_secs_f64() * self.rate as f64) as u64;

        if tokens_to_add > 0 {
            let current = self.tokens.load(Ordering::Relaxed);
            let new_tokens = (current + tokens_to_add).min(self.max_tokens);
            self.tokens.store(new_tokens, Ordering::Relaxed);
            *last = now;
        }
    }
}
