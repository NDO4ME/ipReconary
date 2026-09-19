use regex::Regex;
use reqwest::blocking::Client;
use std::time::Duration;

const RADB_URL: &str = "https://www.radb.net/query";

pub fn fetch_cidrs_from_asn(asn: &str) -> Result<Vec<String>, String> {
    let asn_clean = asn.trim().to_uppercase();
    let asn_query = if asn_clean.starts_with("AS") {
        asn_clean.clone()
    } else {
        format!("AS{}", asn_clean)
    };

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let url = format!(
        "{}?advanced_query=1&keywords={}&-T+option=&ip_option=&-i=1&-i+option=origin",
        RADB_URL, asn_query
    );

    let response = client
        .get(&url)
        .send()
        .map_err(|e| format!("Failed to query RADB: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("RADB returned status: {}", response.status()));
    }

    let body = response
        .text()
        .map_err(|e| format!("Failed to read RADB response: {}", e))?;

    let cidrs = parse_cidrs(&body)?;

    if cidrs.is_empty() {
        return Err(format!("No routes found for {}", asn_query));
    }

    Ok(cidrs)
}

fn parse_cidrs(html: &str) -> Result<Vec<String>, String> {
    // Regex to match route: and route6: CIDR blocks
    let re = Regex::new(r"route6?:\s+([0-9a-fA-F:\.\/]+)")
        .map_err(|e| format!("Regex error: {}", e))?;

    let mut cidrs: Vec<String> = re
        .captures_iter(html)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect();

    // Remove duplicates
    cidrs.sort();
    cidrs.dedup();

    Ok(cidrs)
}
