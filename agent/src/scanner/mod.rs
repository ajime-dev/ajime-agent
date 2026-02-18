//! Local network scanner using pure async TCP probing.
//!
//! No external binaries (nmap, ping) are required. Concurrency is bounded
//! by a semaphore to avoid flooding the network interface.

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use ipnet::Ipv4Net;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::sync::Semaphore;
use tracing::{debug, info};

/// Ports probed on each candidate host.
const PROBE_PORTS: &[u16] = &[22, 80, 8080];

/// Max concurrent TCP probes to avoid overwhelming the local network.
const MAX_CONCURRENT: usize = 64;

/// Per-probe timeout.
const PROBE_TIMEOUT_MS: u64 = 500;

/// A device discovered during a subnet scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredDevice {
    /// IPv4 address of the host.
    pub ip: String,

    /// Ports that accepted a TCP connection.
    pub open_ports: Vec<u16>,

    /// True when port 8080 is open, indicating a running Ajime agent.
    pub has_agent: bool,
}

/// Scan all hosts in `cidr` (e.g. `"192.168.1.0/24"`) and return reachable devices.
///
/// The scan is best-effort: hosts that do not respond within the timeout are
/// silently skipped.
pub async fn scan_subnet(cidr: &str) -> Vec<DiscoveredDevice> {
    let net: Ipv4Net = match cidr.parse() {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!("Invalid CIDR {}: {}", cidr, e);
            return vec![];
        }
    };

    let hosts: Vec<IpAddr> = net.hosts().map(IpAddr::V4).collect();
    info!("Scanning {} hosts in {}", hosts.len(), cidr);

    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT));
    let mut handles = Vec::with_capacity(hosts.len());

    for ip in hosts {
        let sem = Arc::clone(&semaphore);
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.ok()?;
            let open_ports = probe_ports(ip).await;
            if open_ports.is_empty() {
                return None;
            }
            let has_agent = open_ports.contains(&8080);
            Some(DiscoveredDevice {
                ip: ip.to_string(),
                open_ports,
                has_agent,
            })
        }));
    }

    let mut results = Vec::new();
    for handle in handles {
        if let Ok(Some(device)) = handle.await {
            debug!("Found device: {} ports={:?}", device.ip, device.open_ports);
            results.push(device);
        }
    }

    info!("Scan complete: {} devices found", results.len());
    results
}

/// Probe a set of ports on `ip` and return those that accepted a connection.
async fn probe_ports(ip: IpAddr) -> Vec<u16> {
    let mut open = Vec::new();
    let timeout = Duration::from_millis(PROBE_TIMEOUT_MS);

    for &port in PROBE_PORTS {
        let addr = SocketAddr::new(ip, port);
        if let Ok(Ok(_)) = tokio::time::timeout(timeout, TcpStream::connect(addr)).await {
            open.push(port);
        }
    }

    open
}
