use reqwest::Client;
use scraper::{Html, Selector};
use std::time::Duration;
use tokio::time::sleep;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
const MAX_BODY_SIZE: usize = 1024 * 1024; // 1MB
const MAX_RETRIES: u32 = 2;

pub struct HttpClient {
    client: Client,
    pub timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct PageInfo {
    pub title: Option<String>,
    pub status: u16,
    pub hostname: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReferenceData {
    pub title: Option<String>,
    pub favicon_hash: Option<String>,
}

impl HttpClient {
    pub fn new(timeout_ms: u64) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .connect_timeout(Duration::from_millis(timeout_ms))
            .danger_accept_invalid_certs(true)
            .user_agent(USER_AGENT)
            .redirect(reqwest::redirect::Policy::limited(5))
            .gzip(true)
            .brotli(true)
            .deflate(true)
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            client,
            timeout: Duration::from_millis(timeout_ms),
        })
    }

    pub async fn fetch_reference(&self, domain: &str) -> Result<ReferenceData, String> {
        let base_url = if domain.starts_with("http://") || domain.starts_with("https://") {
            domain.to_string()
        } else {
            format!("https://{}", domain)
        };

        // Try to fetch favicon first
        let favicon_hash = self.fetch_favicon_hash(&base_url).await;

        // Fetch title
        let title = self.fetch_title(&base_url).await;

        if favicon_hash.is_none() && title.is_none() {
            return Err("Could not fetch favicon or title from domain".to_string());
        }

        Ok(ReferenceData {
            title,
            favicon_hash,
        })
    }

    async fn fetch_favicon_hash(&self, base_url: &str) -> Option<String> {
        let urls = vec![
            format!("{}/favicon.ico", base_url.trim_end_matches('/')),
            format!("{}/favicon.png", base_url.trim_end_matches('/')),
        ];

        for url in urls {
            if let Ok(hash) = self.try_fetch_favicon_with_retry(&url).await {
                return Some(hash);
            }
        }

        // Try HTTP if HTTPS failed
        if base_url.starts_with("https://") {
            let http_base = base_url.replace("https://", "http://");
            let url = format!("{}/favicon.ico", http_base.trim_end_matches('/'));
            if let Ok(hash) = self.try_fetch_favicon_with_retry(&url).await {
                return Some(hash);
            }
        }

        None
    }

    async fn try_fetch_favicon_with_retry(&self, url: &str) -> Result<String, ()> {
        for attempt in 0..MAX_RETRIES {
            match self.try_fetch_favicon(url).await {
                Ok(hash) => return Ok(hash),
                Err(_) if attempt < MAX_RETRIES - 1 => {
                    // Exponential backoff: attempt * 200ms
                    let delay = (attempt + 1) * 200;
                    sleep(Duration::from_millis(delay as u64)).await;
                }
                Err(_) => return Err(()),
            }
        }
        Err(())
    }

    async fn try_fetch_favicon(&self, url: &str) -> Result<String, ()> {
        let resp = self.client.get(url).send().await.map_err(|_| ())?;

        if !resp.status().is_success() {
            return Err(());
        }

        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !content_type.contains("image") && !content_type.contains("octet-stream") {
            return Err(());
        }

        let bytes = resp.bytes().await.map_err(|_| ())?;
        if bytes.is_empty() || bytes.len() > MAX_BODY_SIZE {
            return Err(());
        }

        let hash = format!("{:x}", md5::compute(&bytes));
        Ok(hash)
    }

    async fn fetch_title(&self, base_url: &str) -> Option<String> {
        if let Ok(title) = self.try_fetch_title_with_retry(base_url).await {
            return Some(title);
        }

        // Try HTTP if HTTPS failed
        if base_url.starts_with("https://") {
            let http_base = base_url.replace("https://", "http://");
            if let Ok(title) = self.try_fetch_title_with_retry(&http_base).await {
                return Some(title);
            }
        }

        None
    }

    async fn try_fetch_title_with_retry(&self, url: &str) -> Result<String, ()> {
        for attempt in 0..MAX_RETRIES {
            match self.try_fetch_title(url).await {
                Ok(title) => return Ok(title),
                Err(_) if attempt < MAX_RETRIES - 1 => {
                    let delay = (attempt + 1) * 200;
                    sleep(Duration::from_millis(delay as u64)).await;
                }
                Err(_) => return Err(()),
            }
        }
        Err(())
    }

    async fn try_fetch_title(&self, url: &str) -> Result<String, ()> {
        let resp = self.client.get(url).send().await.map_err(|_| ())?;

        if !resp.status().is_success() {
            return Err(());
        }

        let body = resp.text().await.map_err(|_| ())?;
        if body.len() > MAX_BODY_SIZE {
            return Err(());
        }

        extract_title(&body).ok_or(())
    }

    pub async fn scan_ip(&self, ip: &str, reference: &ReferenceData) -> Option<PageInfo> {
        // Try HTTPS first, then HTTP
        let urls = vec![format!("https://{}", ip), format!("http://{}", ip)];

        for url in urls {
            for attempt in 0..MAX_RETRIES {
                match self.try_scan(&url, reference).await {
                    Ok(Some(info)) => return Some(info),
                    Ok(None) => break, // No match, try next protocol
                    Err(_) if attempt < MAX_RETRIES - 1 => {
                        let delay = (attempt + 1) * 200;
                        sleep(Duration::from_millis(delay as u64)).await;
                    }
                    Err(_) => break,
                }
            }
        }

        None
    }

    async fn try_scan(&self, url: &str, reference: &ReferenceData) -> Result<Option<PageInfo>, ()> {
        let resp = self.client.get(url).send().await.map_err(|_| ())?;
        let status = resp.status().as_u16();

        // Extract hostname from response headers if available
        let hostname = resp
            .headers()
            .get("x-served-by")
            .or_else(|| resp.headers().get("server"))
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let body = resp.text().await.map_err(|_| ())?;
        if body.len() > MAX_BODY_SIZE {
            return Err(());
        }

        let title = extract_title(&body);

        // Check title match
        if let Some(ref ref_title) = reference.title {
            if let Some(ref page_title) = title {
                if titles_match(ref_title, page_title) {
                    return Ok(Some(PageInfo {
                        title,
                        status,
                        hostname,
                    }));
                }
            }
        }

        // Check favicon match if we have reference hash
        if let Some(ref ref_hash) = reference.favicon_hash {
            let base_url = url.trim_end_matches('/');
            if let Ok(page_hash) = self
                .try_fetch_favicon(&format!("{}/favicon.ico", base_url))
                .await
            {
                if &page_hash == ref_hash {
                    return Ok(Some(PageInfo {
                        title,
                        status,
                        hostname,
                    }));
                }
            }
        }

        Ok(None)
    }
}

fn extract_title(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("title").ok()?;

    document.select(&selector).next().map(|el| {
        el.text()
            .collect::<String>()
            .trim()
            .to_string()
    })
}

fn titles_match(reference: &str, candidate: &str) -> bool {
    let ref_lower = reference.to_lowercase();
    let cand_lower = candidate.to_lowercase();

    // Exact match
    if ref_lower == cand_lower {
        return true;
    }

    // Contains match
    cand_lower.contains(&ref_lower) || ref_lower.contains(&cand_lower)
}

impl Clone for HttpClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            timeout: self.timeout,
        }
    }
}
