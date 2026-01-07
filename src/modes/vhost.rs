//! Virtual host enumeration mode

use crate::cli::VhostArgs;
use crate::core::{load_wordlist, parse_headers};
use crate::error::Result;
use crate::output::{print_error, print_vhost_result, OutputHandler, ProgressTracker, VhostResult};
use futures::stream::{self, StreamExt};
use reqwest::ClientBuilder;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

/// Run virtual host enumeration
pub async fn run(args: VhostArgs) -> Result<()> {
    // Parse exclude lengths
    let exclude_lengths: HashSet<usize> = args
        .exclude_length
        .as_ref()
        .map(|s| s.split(',').filter_map(|l| l.trim().parse().ok()).collect())
        .unwrap_or_default();

    // Build HTTP client
    let mut builder = ClientBuilder::new()
        .user_agent(&args.http.user_agent)
        .timeout(Duration::from_secs(args.http.timeout))
        .danger_accept_invalid_certs(args.http.insecure)
        .pool_max_idle_per_host(100)
        .tcp_nodelay(true);

    if !args.http.follow_redirect {
        builder = builder.redirect(reqwest::redirect::Policy::none());
    }

    if let Some(ref proxy_url) = args.http.proxy {
        builder = builder.proxy(reqwest::Proxy::all(proxy_url)?);
    }

    let client = Arc::new(builder.build()?);

    // Load wordlist
    let wordlist = load_wordlist(&args.global.wordlist)
        .await
        .map_err(crate::error::RbusterError::WordlistError)?;
    let total = wordlist.len();

    // Get base domain for appending
    let base_domain = args.domain.clone();

    // Create progress tracker
    let progress = ProgressTracker::new(total as u64, args.global.quiet || args.global.no_progress);

    // Create output handler
    let output = OutputHandler::new(args.global.output.as_deref()).await?;
    let output = Arc::new(output);

    // Get baseline response for comparison
    let baseline_size = {
        let resp = client.get(&args.url).send().await?;
        resp.bytes().await?.len()
    };

    // Create semaphore for concurrency control
    let semaphore = Arc::new(Semaphore::new(args.global.threads));
    let delay = args.global.delay.map(Duration::from_millis);

    let headers = parse_headers(&args.http.headers);
    let verbose = args.global.verbose;
    let url = args.url.clone();
    let append_domain = args.append_domain;

    // Process vhosts concurrently
    let _results: Vec<_> = stream::iter(wordlist)
        .map(|word| {
            let semaphore = Arc::clone(&semaphore);
            let client = Arc::clone(&client);
            let progress = progress.clone();
            let output = Arc::clone(&output);
            let url = url.clone();
            let headers = headers.clone();
            let exclude_lengths = exclude_lengths.clone();
            let base_domain = base_domain.clone();

            async move {
                let _permit = semaphore.acquire().await.unwrap();

                if let Some(d) = delay {
                    tokio::time::sleep(d).await;
                }

                // Build the host header value
                let host = if append_domain {
                    if let Some(ref domain) = base_domain {
                        format!("{}.{}", word, domain)
                    } else {
                        word.clone()
                    }
                } else {
                    word.clone()
                };

                // Build request with Host header
                let mut request = client.get(&url).header("Host", &host);

                // Add custom headers
                for (key, value) in &headers {
                    request = request.header(key.as_str(), value.as_str());
                }

                let result = request.send().await;
                progress.inc();

                match result {
                    Ok(response) => {
                        let status = response.status().as_u16();
                        let body = response.bytes().await.unwrap_or_default();
                        let size = body.len();

                        // Skip if size matches baseline or is in exclude list
                        let should_show = size != baseline_size
                            && !exclude_lengths.contains(&size)
                            && status != 400; // Skip bad request errors

                        if should_show {
                            progress.inc_found();

                            // Print to console
                            print_vhost_result(&host, status, size);

                            // Write to file if configured
                            if let Some(writer) = output.file_writer() {
                                let result = VhostResult {
                                    host: host.clone(),
                                    status,
                                    size,
                                };
                                if writer.is_json() {
                                    let _ = writer.write_json(&result).await;
                                } else {
                                    let line =
                                        format!("{} (Status: {}) [Size: {}]", host, status, size);
                                    let _ = writer.write_line(&line).await;
                                }
                            }

                            Some((host, status, size))
                        } else {
                            None
                        }
                    }
                    Err(e) => {
                        if verbose {
                            print_error(&format!("{}: {}", host, e), true);
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
