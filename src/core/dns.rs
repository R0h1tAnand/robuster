//! DNS resolver wrapper using hickory-resolver

use crate::error::{RbusterError, Result};
use hickory_resolver::config::{NameServerConfig, Protocol, ResolverConfig, ResolverOpts};
use hickory_resolver::TokioAsyncResolver;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::time::Duration;

/// DNS client configuration
#[derive(Clone, Debug)]
pub struct DnsConfig {
    pub resolver: Option<String>,
    pub timeout: Duration,
}

impl Default for DnsConfig {
    fn default() -> Self {
        Self {
            resolver: None,
            timeout: Duration::from_secs(5),
        }
    }
}

/// DNS resolution result
#[derive(Debug, Clone)]
pub struct DnsResult {
    pub subdomain: String,
    pub ips: Vec<IpAddr>,
    pub cnames: Vec<String>,
}

/// DNS resolver client
pub struct DnsClient {
    resolver: TokioAsyncResolver,
}

impl DnsClient {
    pub async fn new(config: DnsConfig) -> Result<Self> {
        let resolver = if let Some(ref resolver_addr) = config.resolver {
            // Parse custom resolver address
            let socket_addr =
                parse_resolver_address(resolver_addr).map_err(RbusterError::DnsError)?;

            let mut opts = ResolverOpts::default();
            opts.timeout = config.timeout;
            opts.attempts = 2;

            let name_server = NameServerConfig::new(socket_addr, Protocol::Udp);
            let resolver_config = ResolverConfig::from_parts(None, vec![], vec![name_server]);

            TokioAsyncResolver::tokio(resolver_config, opts)
        } else {
            // Use system resolver
            TokioAsyncResolver::tokio_from_system_conf()
                .map_err(|e| RbusterError::DnsError(e.to_string()))?
        };

        Ok(Self { resolver })
    }

    /// Resolve a subdomain and return IPs and CNAMEs
    pub async fn resolve(&self, domain: &str) -> Result<DnsResult> {
        let mut ips = Vec::new();
        let mut cnames = Vec::new();

        // Try to resolve A records
        if let Ok(response) = self.resolver.lookup_ip(domain).await {
            for ip in response.iter() {
                ips.push(ip);
            }
        }

        // Try to resolve CNAME records
        if let Ok(response) = self
            .resolver
            .lookup(domain, hickory_resolver::proto::rr::RecordType::CNAME)
            .await
        {
            for record in response.iter() {
                if let Some(cname) = record.as_cname() {
                    cnames.push(cname.to_utf8());
                }
            }
        }

        if ips.is_empty() && cnames.is_empty() {
            return Err(RbusterError::DnsError(format!(
                "No records found for {}",
                domain
            )));
        }

        Ok(DnsResult {
            subdomain: domain.to_string(),
            ips,
            cnames,
        })
    }

    /// Check if a subdomain exists (simple check)
    pub async fn exists(&self, domain: &str) -> bool {
        self.resolver.lookup_ip(domain).await.is_ok()
    }

    /// Detect wildcard DNS
    pub async fn detect_wildcard(&self, base_domain: &str) -> Option<Vec<IpAddr>> {
        // Test with a random subdomain that shouldn't exist
        let random_subdomain = format!("rbuster-wildcard-test-{}.{}", rand_string(16), base_domain);

        if let Ok(response) = self.resolver.lookup_ip(&random_subdomain).await {
            let ips: Vec<IpAddr> = response.iter().collect();
            if !ips.is_empty() {
                return Some(ips);
            }
        }
        None
    }
}

/// Parse resolver address in format "IP" or "IP:port"
fn parse_resolver_address(addr: &str) -> std::result::Result<SocketAddr, String> {
    if addr.contains(':') {
        SocketAddr::from_str(addr)
            .map_err(|e| format!("Invalid resolver address '{}': {}", addr, e))
    } else {
        IpAddr::from_str(addr)
            .map(|ip| SocketAddr::new(ip, 53))
            .map_err(|e| format!("Invalid resolver IP '{}': {}", addr, e))
    }
}

/// Generate random string for wildcard detection
fn rand_string(len: usize) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyz0123456789".chars().collect();
    let mut result = String::with_capacity(len);
    let mut n = seed as usize;

    for _ in 0..len {
        result.push(chars[n % chars.len()]);
        n = n.wrapping_mul(1103515245).wrapping_add(12345);
    }

    result
}
