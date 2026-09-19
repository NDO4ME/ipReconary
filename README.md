# ipReconary

High-performance network reconnaissance tool for discovering origin IPs behind CDN/WAF services.

This project is a complete rewrite and enhanced version of [ipmap](https://github.com/sercanarga/ipmap), rebuilt from the ground up in Rust with a focus on performance, stability, and simplicity.

## Features

- **ASN-based scanning**: Automatically fetch CIDR blocks from ASN via RADB
- **Subnet scanning**: Direct CIDR range scanning support
- **Favicon hash matching**: MD5 hash comparison for accurate identification
- **Title matching**: HTML title tag comparison with flexible matching
- **High concurrency**: Async I/O with configurable worker pool
- **Rate limiting**: Token bucket algorithm for controlled scanning
- **WAF bypass**: Built-in jitter to avoid detection patterns
- **Graceful shutdown**: Ctrl+C handling with result preservation

## Installation

### From Source

```bash
git clone https://github.com/NDO4ME/ipReconary.git
cd ipReconary
cargo build --release
```

Binary will be available at `./target/release/ipReconary`

### Requirements

- Rust 1.70+

## Usage

```bash
ipReconary [OPTIONS]
```

### Required Parameters

| Parameter | Description |
|-----------|-------------|
| `-a, --asn <ASN>` | ASN number (e.g., AS13335) |
| `-s, --subnet <SUBNET>` | CIDR blocks, comma-separated (e.g., 1.1.1.0/24,2.2.2.0/24) |
| `-d, --domain <DOMAIN>` | Target domain to search for |

> **Note**: Either `--asn` or `--subnet` must be provided (not both). `--domain` is always required.

### Optional Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `--title <TITLE>` | Manual title search (bypasses domain fetch) | - |
| `-w, --workers <N>` | Number of concurrent workers | 100 |
| `-t, --timeout <MS>` | Request timeout in milliseconds | 2000 |
| `-r, --rate <N>` | Rate limit (requests/second, 0 = unlimited) | 0 |
| `--no-jitter` | Disable request jitter | - |
| `-v, --verbose` | Enable verbose logging | - |

## Examples

### Scan ASN for a domain

```bash
ipReconary -a AS13335 -d cloudflare.com
```

### Scan specific subnets

```bash
ipReconary -s 104.16.0.0/16,172.67.0.0/16 -d example.com
```

### Manual title search (for DDoS-protected sites)

```bash
ipReconary -a AS13335 -d protected-site.com --title "Login Portal"
```

### Rate-limited scan with custom workers

```bash
ipReconary -a AS13335 -d example.com -w 50 -r 100 -t 5000
```

## Output

Results are written to `ipReconary.out` in the current directory:

```
==================== RESULT ====================
Method: Domain by ASN
Search Site: example.com
Timeout: 2000ms
IP Blocks: 104.16.0.0/16, 172.67.0.0/16

Founded Websites:
[200] | 104.16.132.229 | Example Domain | cloudflare
[200] | 104.16.133.229 | Example Domain | cloudflare
================================================
```

## How It Works

1. **Reference Collection**: Fetches favicon hash and/or HTML title from target domain
2. **IP Enumeration**: Expands ASN routes or CIDR blocks into individual IPs (max 1M)
3. **Parallel Scanning**: Tests each IP with configurable concurrency
4. **Matching**: Compares favicon hash or title against reference data
5. **Result Aggregation**: Collects and outputs all matching IPs

## Interrupt Handling

- **First Ctrl+C**: Graceful shutdown, waits for active requests, writes results
- **Second Ctrl+C**: Force exit

## Limitations

- Maximum 1,000,000,000 IPs per scan (1 billion)
- IPv4 only (IPv6 is skipped)
- SSL certificate validation disabled (required for IP-based scanning)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Author

**NDO4ME** - 2026

## Acknowledgments

Inspired by [ipmap](https://github.com/sercanarga/ipmap) by Sercan Arga.
