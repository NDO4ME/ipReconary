use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(name = "ipReconary")]
#[command(about = "Network discovery tool - find real IPs behind CDN/WAF")]
pub struct Args {
    /// ASN number (e.g., AS13335)
    #[arg(long = "asn", short = 'a')]
    pub asn: Option<String>,

    /// CIDR subnets comma-separated (e.g., 1.1.1.0/24,2.2.2.0/24)
    #[arg(long = "subnet", short = 's')]
    pub subnet: Option<String>,

    /// Target domain (REQUIRED) - fetch favicon/title from this domain
    #[arg(long = "domain", short = 'd')]
    pub domain: Option<String>,

    /// Manual title search (skip fetching from domain, use when site has DDoS protection)
    #[arg(long = "title")]
    pub title: Option<String>,

    /// Number of concurrent workers
    #[arg(long = "workers", short = 'w', default_value = "100")]
    pub workers: usize,

    /// Timeout in milliseconds
    #[arg(long = "timeout", short = 't', default_value = "2000")]
    pub timeout: u64,

    /// Rate limit (requests per second, 0 = unlimited)
    #[arg(long = "rate", short = 'r', default_value = "0")]
    pub rate: u64,

    /// Disable jitter (jitter is enabled by default with 10-50ms delay)
    #[arg(long = "no-jitter")]
    pub no_jitter: bool,

    /// Verbose mode (detailed logging)
    #[arg(long = "verbose", short = 'v')]
    pub verbose: bool,
}

impl Args {
    pub fn validate(&self) -> Result<(), String> {
        // Must have ASN or subnet (not both)
        if self.asn.is_none() && self.subnet.is_none() {
            return Err("Either --asn or --subnet must be provided".to_string());
        }
        if self.asn.is_some() && self.subnet.is_some() {
            return Err("Cannot use both --asn and --subnet together".to_string());
        }

        // Domain is REQUIRED
        if self.domain.is_none() {
            return Err("--domain is required (the domain you're searching for)".to_string());
        }

        if self.workers == 0 {
            return Err("Workers must be greater than 0".to_string());
        }
        Ok(())
    }
}
