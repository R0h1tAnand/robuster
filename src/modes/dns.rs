//! DNS subdomain enumeration mode

use crate::cli::DnsArgs;
use crate::core::{load_wordlist, DnsClient, DnsConfig};
use crate::error::Result;
use crate::output::{
    print_dns_result, print_error, print_warning, DnsResultJson, OutputHandler, ProgressTracker,
};
use futures::stream::{self, StreamExt};
use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

/// Run DNS subdomain enumeration
pub async fn run(args: DnsArgs) -> Result<()> {
    // Create DNS client
    let dns_config = DnsConfig {
        resolver: args.resolver.clone(),
        timeout: Duration::from_secs(args.timeout),
    };
    let dns_client = Arc::new(DnsClient::new(dns_config).await?);

    // Load wordlist
    let wordlist = load_wordlist(&args.global.wordlist)
        .await
        .map_err(crate::error::RbusterError::WordlistError)?;
    let total = wordlist.len();

    // Normalize domain
    let base_domain = args.domain.trim_start_matches('.').to_string();

    // Create progress tracker
    let progress = ProgressTracker::new(total as u64, args.global.quiet || args.global.no_progress);

    // Create output handler
    let output = OutputHandler::new(args.global.output.as_deref()).await?;
    let output = Arc::new(output);

    // Check for wildcard DNS
    let wildcard_ips: HashSet<IpAddr> = if !args.wildcard {
        if let Some(ips) = dns_client.detect_wildcard(&base_domain).await {
            print_warning(&format!(
                "Wildcard DNS detected! IPs: {}. Use --wildcard to force continue",
                ips.iter()
                    .map(|ip| ip.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            if !args.global.quiet {
                ips.into_iter().collect()
            } else {
                HashSet::new()
            }
        } else {
            HashSet::new()
        }
    } else {
        HashSet::new()
    };

    // Create semaphore for concurrency control
    let semaphore = Arc::new(Semaphore::new(args.global.threads));
    let delay = args.global.delay.map(Duration::from_millis);

    let show_ips = args.show_ips;
    let show_cname = args.show_cname;
    let verbose = args.global.verbose;

    // Process subdomains concurrently
    let _results: Vec<_> = stream::iter(wordlist)
        .map(|word| {
            let semaphore = Arc::clone(&semaphore);
            let dns_client = Arc::clone(&dns_client);
            let progress = progress.clone();
            let output = Arc::clone(&output);
            let base_domain = base_domain.clone();
            let wildcard_ips = wildcard_ips.clone();

            async move {
                let _permit = semaphore.acquire().await.unwrap();

                if let Some(d) = delay {
                    tokio::time::sleep(d).await;
                }

                let subdomain = format!("{}.{}", word, base_domain);
                let result = dns_client.resolve(&subdomain).await;
                progress.inc();

                match result {
                    Ok(dns_result) => {
                        // Check if this is a wildcard response
                        let is_wildcard = !wildcard_ips.is_empty()
                            && dns_result.ips.iter().all(|ip| wildcard_ips.contains(ip));

                        if !is_wildcard {
                            progress.inc_found();

                            // Print to console
                            print_dns_result(
                                &subdomain,
                                &dns_result.ips,
                                &dns_result.cnames,
                                show_ips,
                                show_cname,
                            );

                            // Write to file if configured
                            if let Some(writer) = output.file_writer() {
                                let result = DnsResultJson {
                                    subdomain: subdomain.clone(),
                                    ips: dns_result.ips.iter().map(|ip| ip.to_string()).collect(),
                                    cnames: dns_result.cnames.clone(),
                                };
                                if writer.is_json() {
                                    let _ = writer.write_json(&result).await;
                                } else {
                                    let ips_str = dns_result
                                        .ips
                                        .iter()
                                        .map(|ip| ip.to_string())
                                        .collect::<Vec<_>>()
                                        .join(", ");
                                    let line = format!("{} [{}]", subdomain, ips_str);
                                    let _ = writer.write_line(&line).await;
                                }
                            }

                            Some(dns_result)
                        } else {
                            None
                        }
                    }
                    Err(e) => {
                        if verbose {
                            print_error(&format!("{}: {}", subdomain, e), true);
                        }
                        None
                    }
                }
            }
        })
        .buffer_unordered(args.global.threads)
        .collect()
        .await;

    progress.finish();
    output.finalize().await?;

    Ok(())
}
