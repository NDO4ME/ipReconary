use crate::scanner::ScanResult;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;

const OUTPUT_FILE: &str = "ipReconary.out";

pub struct OutputWriter {
    file: Mutex<File>,
    verbose: bool,
}

pub struct ResultInfo {
    pub method: String,
    pub search_site: Option<String>,
    pub timeout: u64,
    pub ip_blocks: Vec<String>,
    pub interrupted: bool,
}

impl OutputWriter {
    pub fn new(verbose: bool) -> Result<Self, String> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(OUTPUT_FILE)
            .map_err(|e| format!("Failed to create output file: {}", e))?;

        Ok(Self {
            file: Mutex::new(file),
            verbose,
        })
    }

    pub fn write_log(&self, message: &str) {
        if self.verbose {
            let mut file = self.file.lock().unwrap();
            writeln!(file, "[LOG] {}", message).ok();
            file.flush().ok();
        }
    }

    pub fn write_result(&self, info: &ResultInfo, results: &[ScanResult]) {
        let mut file = self.file.lock().unwrap();

        writeln!(file, "==================== RESULT ====================").ok();

        // Method
        let method = if info.interrupted {
            "Interrupted"
        } else if info.method == "ASN" {
            if info.search_site.is_some() {
                "Domain by ASN"
            } else {
                "Search All ASN"
            }
        } else if info.search_site.is_some() {
            "Domain by IP"
        } else {
            "IP"
        };
        writeln!(file, "Method: {}", method).ok();

        // Search Site
        if let Some(ref site) = info.search_site {
            writeln!(file, "Search Site: {}", site).ok();
        }

        // Timeout
        writeln!(file, "Timeout: {}ms", info.timeout).ok();

        // IP Blocks
        writeln!(file, "IP Blocks: {}", info.ip_blocks.join(", ")).ok();

        writeln!(file).ok();
        writeln!(file, "Founded Websites:").ok();

        if results.is_empty() {
            writeln!(file, "(none)").ok();
        } else {
            for result in results {
                let status = result.info.status;
                let ip = result.ip;
                let title = result.info.title.as_deref().unwrap_or("-");
                let hostname = result.info.hostname.as_deref().unwrap_or("-");

                writeln!(file, "[{}] | {} | {} | {}", status, ip, title, hostname).ok();
            }
        }

        writeln!(file, "================================================").ok();
        file.flush().ok();
    }

}
