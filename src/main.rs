mod asn;
mod cli;
mod http;
mod jitter;
mod network;
mod output;
mod ratelimit;
mod scanner;
mod signal;

use clap::Parser;
use cli::Args;
use http::{HttpClient, ReferenceData};
use output::{OutputWriter, ResultInfo};
use scanner::Scanner;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if let Err(e) = args.validate() {
        eprintln!("[ERROR] {}", e);
        std::process::exit(1);
    }

    if let Err(e) = run(args).await {
        eprintln!("[ERROR] {}", e);
        std::process::exit(1);
    }
}

async fn run(args: Args) -> Result<(), String> {
    let stop_flag = Arc::new(AtomicBool::new(false));
    signal::setup_handler(stop_flag.clone());

    let output = Arc::new(OutputWriter::new(args.verbose)?);

    // Determine mode
    let mode = if args.asn.is_some() {
        "ASN".to_string()
    } else {
        "SUBNET".to_string()
    };

    // Domain is required (validated in cli.rs)
    let domain = args.domain.as_ref().unwrap();

    eprintln!("[ipReconary] Starting...");
    eprintln!("[*] Target domain: {}", domain);

    // Get CIDRs
    let cidrs = if let Some(ref asn) = args.asn {
        eprintln!("[*] Fetching routes for {}...", asn);
        asn::fetch_cidrs_from_asn(asn)?
    } else if let Some(ref subnet_list) = args.subnet {
        network::parse_cidr_list(subnet_list)
    } else {
        return Err("No IP source specified".to_string());
    };

    eprintln!("[*] Found {} CIDR blocks", cidrs.len());
    output.write_log(&format!("CIDR blocks: {:?}", cidrs));

    // Expand to IPs
    eprintln!("[*] Expanding CIDRs to IP list...");
    let ips = network::expand_cidrs(&cidrs)?;
    eprintln!("[*] Total IPs to scan: {}", ips.len());
    output.write_log(&format!("Total IPs: {}", ips.len()));

    // Get reference data
    // If --title is provided, use it directly (skip domain fetch - useful for DDoS protected sites)
    // Otherwise, fetch favicon/title from domain
    let reference = if let Some(ref title) = args.title {
        eprintln!("[*] Using manual title (skipping domain fetch): {}", title);
        ReferenceData {
            title: Some(title.clone()),
            favicon_hash: None,
        }
    } else {
        eprintln!("[*] Fetching reference from {}...", domain);
        let client = HttpClient::new(args.timeout)?;
        let ref_data = client.fetch_reference(domain).await?;

        if let Some(ref hash) = ref_data.favicon_hash {
            eprintln!("[*] Got favicon hash: {}", hash);
            output.write_log(&format!("Favicon hash: {}", hash));
        }
        if let Some(ref title) = ref_data.title {
            eprintln!("[*] Got title: {}", title);
            output.write_log(&format!("Title: {}", title));
        }

        ref_data
    };

    // Jitter is enabled by default, --no-jitter disables it
    let jitter_enabled = !args.no_jitter;

    // Create scanner
    let scanner = Scanner::new(
        args.timeout,
        args.workers,
        args.verbose,
        args.rate,
        jitter_enabled,
    )?;

    let rate_info = if args.rate == 0 {
        "unlimited".to_string()
    } else {
        format!("{}/s", args.rate)
    };

    eprintln!(
        "[*] Starting scan: {} workers, {}ms timeout, rate: {}, jitter: {}",
        args.workers,
        args.timeout,
        rate_info,
        if jitter_enabled { "on" } else { "off" }
    );

    // Run scan
    let results = scanner
        .scan(ips.clone(), reference, stop_flag.clone())
        .await;

    // Check if interrupted
    let interrupted = stop_flag.load(Ordering::Relaxed);

    // Write result file
    let result_info = ResultInfo {
        method: mode,
        search_site: args.domain.clone(),
        timeout: args.timeout,
        ip_blocks: cidrs,
        interrupted,
    };

    output.write_result(&result_info, &results);

    if results.is_empty() {
        eprintln!("[!] No matches found.");
    } else {
        eprintln!("[+] {} match(es) found. See ipReconary.out", results.len());
    }

    Ok(())
}
