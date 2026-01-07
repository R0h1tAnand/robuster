//! Fuzzing mode with FUZZ keyword replacement

use crate::cli::FuzzArgs;
use crate::core::{load_wordlist, parse_headers};
use crate::error::Result;
use crate::output::{print_error, print_fuzz_result, FuzzResult, OutputHandler, ProgressTracker};
use futures::stream::{self, StreamExt};
use reqwest::{ClientBuilder, Method};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

const FUZZ_KEYWORD: &str = "FUZZ";

/// Run fuzzing mode
pub async fn run(args: FuzzArgs) -> Result<()> {
    // Validate FUZZ keyword is present
    let has_fuzz_in_url = args.url.contains(FUZZ_KEYWORD);
    let has_fuzz_in_headers = args.http.headers.iter().any(|h| h.contains(FUZZ_KEYWORD));
    let has_fuzz_in_data = args
        .data
        .as_ref()
        .map(|d| d.contains(FUZZ_KEYWORD))
        .unwrap_or(false);

    if !has_fuzz_in_url && !has_fuzz_in_headers && !has_fuzz_in_data {
        return Err(crate::error::RbusterError::ConfigError(
            "FUZZ keyword not found in URL, headers, or data".to_string(),
        ));
    }

    // Parse exclude status codes and lengths
    let exclude_status: HashSet<u16> = args
        .exclude_status
        .as_ref()
        .map(|s| s.split(',').filter_map(|c| c.trim().parse().ok()).collect())
        .unwrap_or_default();

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

    // Create progress tracker
    let progress = ProgressTracker::new(total as u64, args.global.quiet || args.global.no_progress);

    // Create output handler
    let output = OutputHandler::new(args.global.output.as_deref()).await?;
    let output = Arc::new(output);

    // Create semaphore for concurrency control
    let semaphore = Arc::new(Semaphore::new(args.global.threads));
    let delay = args.global.delay.map(Duration::from_millis);

    let _base_headers = parse_headers(&args.http.headers);
    let raw_headers = args.http.headers.clone();
    let verbose = args.global.verbose;
    let base_url = args.url.clone();
    let base_data = args.data.clone();
    let method_str = args.http.method.clone();
    let filter_string = args.filter_string.clone();
    let cookies = args.http.cookies.clone();

    // Process payloads concurrently
    let _results: Vec<_> = stream::iter(wordlist)
        .map(|payload| {
            let semaphore = Arc::clone(&semaphore);
            let client = Arc::clone(&client);
            let progress = progress.clone();
            let output = Arc::clone(&output);
            let exclude_status = exclude_status.clone();
            let exclude_lengths = exclude_lengths.clone();
            let base_url = base_url.clone();
            let raw_headers = raw_headers.clone();
            let base_data = base_data.clone();
            let method_str = method_str.clone();
            let filter_string = filter_string.clone();
            let cookies = cookies.clone();

            async move {
                let _permit = semaphore.acquire().await.unwrap();

                if let Some(d) = delay {
                    tokio::time::sleep(d).await;
                }

                // Replace FUZZ keyword
                let url = base_url.replace(FUZZ_KEYWORD, &payload);
                let data = base_data
                    .as_ref()
                    .map(|d| d.replace(FUZZ_KEYWORD, &payload));

                // Build request
                let method = Method::from_bytes(method_str.as_bytes()).unwrap_or(Method::GET);
                let mut request = client.request(method, &url);

                // Replace FUZZ in headers
                for raw_header in &raw_headers {
                    let replaced = raw_header.replace(FUZZ_KEYWORD, &payload);
                    let parts: Vec<&str> = replaced.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        request = request.header(parts[0].trim(), parts[1].trim());
                    }
                }

                // Add cookies
                if let Some(ref c) = cookies {
                    request = request.header("Cookie", c.replace(FUZZ_KEYWORD, &payload));
                }

                // Add body if present
                if let Some(ref body) = data {
                    request = request.body(body.clone());
                    if method_str == "POST" {
                        request =
                            request.header("Content-Type", "application/x-www-form-urlencoded");
                    }
                }

                let result = request.send().await;
                progress.inc();

                match result {
                    Ok(response) => {
                        let status = response.status().as_u16();
                        let body = response.text().await.unwrap_or_default();
                        let size = body.len();
                        let words = body.split_whitespace().count();
                        let lines = body.lines().count();

                        // Apply filters
                        let mut should_show =
                            !exclude_status.contains(&status) && !exclude_lengths.contains(&size);

                        // Apply string filter
                        if let Some(ref filter) = filter_string {
                            if body.contains(filter) {
                                should_show = false;
                            }
                        }

                        if should_show {
                            progress.inc_found();

                            // Print to console
                            print_fuzz_result(&payload, status, size, words, lines);

                            // Write to file if configured
                            if let Some(writer) = output.file_writer() {
                                let result = FuzzResult {
                                    payload: payload.clone(),
                                    status,
                                    size,
                                    words,
                                    lines,
                                };
                                if writer.is_json() {
                                    let _ = writer.write_json(&result).await;
                                } else {
                                    let line = format!(
                                        "{} [Status: {}, Size: {}, Words: {}, Lines: {}]",
                                        payload, status, size, words, lines
                                    );
                                    let _ = writer.write_line(&line).await;
                                }
                            }

                            Some((payload, status, size))
                        } else {
                            None
                        }
                    }
                    Err(e) => {
                        if verbose {
                            print_error(&format!("{}: {}", payload, e), true);
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
