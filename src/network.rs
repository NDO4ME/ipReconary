use ipnetwork::IpNetwork;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::net::IpAddr;

const MAX_IPS: usize = 1_000_000_000;

pub fn expand_cidrs(cidrs: &[String]) -> Result<Vec<IpAddr>, String> {
    let mut ips = Vec::new();
    let mut total_estimate: u128 = 0;

    // First pass: estimate total (IPv4 only, ignore IPv6 silently)
    for cidr in cidrs {
        let network: IpNetwork = cidr
            .trim()
            .parse()
            .map_err(|e| format!("Invalid CIDR '{}': {}", cidr, e))?;

        // Only count IPv4 addresses, silently ignore IPv6
        if let IpNetwork::V4(net) = network {
            total_estimate += net.size() as u128;
        }
    }

    if total_estimate > MAX_IPS as u128 {
        return Err(format!(
            "Too many IPs: {} (max {}). Use smaller subnets.",
            total_estimate, MAX_IPS
        ));
    }

    // Second pass: collect IPs (IPv4 only)
    for cidr in cidrs {
        let network: IpNetwork = cidr.trim().parse().unwrap();
        // Skip IPv6 networks
        if let IpNetwork::V4(_) = network {
            for ip in network.iter() {
                ips.push(ip);
            }
        }
    }

    // Shuffle for better distribution
    let mut rng = thread_rng();
    ips.shuffle(&mut rng);

    Ok(ips)
}

pub fn parse_cidr_list(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}
