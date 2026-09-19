use crate::http::{HttpClient, PageInfo, ReferenceData};
use crate::jitter::apply_jitter;
use crate::ratelimit::RateLimiter;
use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;

pub struct Scanner {
    client: HttpClient,
    workers: usize,
    verbose: bool,
    rate_limiter: Arc<RateLimiter>,
    jitter_enabled: bool,
}

#[derive(Clone, Debug)]
pub struct ScanResult {
    pub ip: IpAddr,
    pub info: PageInfo,
}

impl Scanner {
    pub fn new(
        timeout_ms: u64,
        workers: usize,
        verbose: bool,
        rate: u64,
        jitter_enabled: bool,
    ) -> Result<Self, String> {
        let client = HttpClient::new(timeout_ms)?;
        let rate_limiter = RateLimiter::new(rate);

        Ok(Self {
            client,
            workers,
            verbose,
            rate_limiter,
            jitter_enabled,
        })
    }

    pub async fn scan(
        &self,
        ips: Vec<IpAddr>,
        reference: ReferenceData,
        stop_flag: Arc<AtomicBool>,
    ) -> Vec<ScanResult> {
        let total = ips.len();
        let scanned = Arc::new(AtomicUsize::new(0));
        let found = Arc::new(AtomicUsize::new(0));
        let results = Arc::new(tokio::sync::Mutex::new(Vec::new()));

        let semaphore = Arc::new(Semaphore::new(self.workers));
        let reference = Arc::new(reference);
        let client = Arc::new(self.client.clone());
        let rate_limiter = self.rate_limiter.clone();

        let mut handles = Vec::new();

        for ip in ips {
            if stop_flag.load(Ordering::Relaxed) {
                break;
            }

            // Apply rate limiting
            rate_limiter.acquire().await;

            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let client = client.clone();
            let reference = reference.clone();
            let stop_flag = stop_flag.clone();
            let scanned = scanned.clone();
            let found = found.clone();
            let results = results.clone();
            let verbose = self.verbose;
            let jitter_enabled = self.jitter_enabled;

            let handle = tokio::spawn(async move {
                let _permit = permit;

                if stop_flag.load(Ordering::Relaxed) {
                    return;
                }

                // Apply jitter for WAF bypass
                if jitter_enabled {
                    apply_jitter().await;
                }

                let ip_str = ip.to_string();
                let result = client.scan_ip(&ip_str, &reference).await;

                let current = scanned.fetch_add(1, Ordering::Relaxed) + 1;

                if let Some(info) = result {
                    found.fetch_add(1, Ordering::Relaxed);

                    let match_line = format!(
                        "[{}] {} | {} | {}",
                        info.status,
                        ip,
                        info.title.as_deref().unwrap_or("-"),
                        info.hostname.as_deref().unwrap_or("-")
                    );

                    eprintln!("\n[MATCH] {}", match_line);

                    results.lock().await.push(ScanResult { ip, info });
                } else if verbose && current % 100 == 0 {
                    eprintln!("[LOG] Scanned {}/{}", current, total);
                }

                // Print progress every 1000 IPs
                if current % 1000 == 0 {
                    let found_count = found.load(Ordering::Relaxed);
                    eprint!("\r[SCAN] {}/{} scanned, {} found", current, total, found_count);
                }
            });

            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            let _ = handle.await;
        }

        let final_scanned = scanned.load(Ordering::Relaxed);
        let final_found = found.load(Ordering::Relaxed);
        eprintln!(
            "\r[DONE] {}/{} scanned, {} found    ",
            final_scanned, total, final_found
        );

        Arc::try_unwrap(results)
            .unwrap()
            .into_inner()
    }
}
