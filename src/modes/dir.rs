//! Directory/file enumeration mode

use crate::cli::DirArgs;
use crate::core::{load_wordlist, parse_headers, HttpClient, HttpConfig};
use crate::error::Result;
use crate::output::{
    print_dir_result, print_error, print_warning, DirResult, OutputHandler, ProgressTracker,
};
use futures::stream::{self, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

/// Backup file extensions to check
const BACKUP_EXTENSIONS: &[&str] = &[
    ".bak", ".backup", ".old", ".orig", ".save", "~", ".swp", ".tmp", ".copy",
];

/// Run directory enumeration
pub async fn run(args: DirArgs) -> Result<()> {
    // Parse configuration
    let extensions = args.parse_extensions();
    let valid_status_codes: std::collections::HashSet<u16> =
        args.parse_status_codes().into_iter().collect();
    let blacklist_codes: std::collections::HashSet<u16> =
        args.parse_status_codes_blacklist().into_iter().collect();
    let exclude_lengths: std::collections::HashSet<usize> =
        args.parse_exclude_lengths().into_iter().collect();

    // Normalize base URL
    let base_url = args.url.trim_end_matches('/').to_string();

    // Create HTTP client
    let http_config = HttpConfig {
        user_agent: args.http.user_agent.clone(),
        timeout: Duration::from_secs(args.http.timeout),
        insecure: args.http.insecure,
        follow_redirect: args.http.follow_redirect,
        proxy: args.http.proxy.clone(),
        headers: parse_headers(&args.http.headers),
        cookies: args.http.cookies.clone(),
        username: args.http.username.clone(),
        password: args.http.password.clone(),
    };
    let http_client = Arc::new(HttpClient::new(http_config)?);

    // Load wordlist
    let wordlist = load_wordlist(&args.global.wordlist)
        .await
        .map_err(crate::error::RbusterError::WordlistError)?;

    // Calculate total requests (wordlist * extensions)
    let ext_multiplier = if extensions.is_empty() {
        1
    } else {
        extensions.len() + 1
    };
    let total_requests = wordlist.len() * ext_multiplier * (if args.add_slash { 2 } else { 1 });

    // Create progress tracker
    let progress = ProgressTracker::new(
        total_requests as u64,
        args.global.quiet || args.global.no_progress,
    );

    // Create output handler
    let output = OutputHandler::new(args.global.output.as_deref()).await?;
    let output = Arc::new(output);

    // Check for wildcard
    if !args.wildcard {
        let random_path = format!("{}/rbuster-wildcard-test-{}", base_url, rand_string(16));
        match http_client.check_url(&random_path, &args.http.method).await {
            Ok((status, _, _)) if valid_status_codes.contains(&status) => {
                print_warning("Wildcard response detected! Use --wildcard to force continue");
                if !args.global.quiet {
                    return Ok(());
                }
            }
            _ => {}
        }
    }

    // Create semaphore for concurrency control
    let semaphore = Arc::new(Semaphore::new(args.global.threads));
    let delay = args.global.delay.map(Duration::from_millis);

    // Generate all URLs to check
    let mut urls_to_check: Vec<String> = Vec::with_capacity(total_requests);
    for word in &wordlist {
        // Base path
        let path = if word.starts_with('/') {
            word.clone()
        } else {
            format!("/{}", word)
        };

        // Add base path
        urls_to_check.push(format!("{}{}", base_url, path));

        // Add with slash if requested
        if args.add_slash && !path.ends_with('/') {
            urls_to_check.push(format!("{}{}/", base_url, path));
        }

        // Add extensions
        for ext in &extensions {
            let ext_path = if ext.starts_with('.') {
                format!("{}{}", path, ext)
            } else {
                format!("{}.{}", path, ext)
            };
            urls_to_check.push(format!("{}{}", base_url, ext_path));
        }
    }

    // Process URLs concurrently
    let method = args.http.method.clone();
    let show_length = args.show_length;
    let expanded = args.expanded;
    let discover_backup = args.discover_backup;
    let verbose = args.global.verbose;

    let results: Vec<_> = stream::iter(urls_to_check)
        .map(|url| {
            let semaphore = Arc::clone(&semaphore);
            let http_client = Arc::clone(&http_client);
            let method = method.clone();
            let progress = progress.clone();
            let output = Arc::clone(&output);
            let valid_status_codes = valid_status_codes.clone();
            let blacklist_codes = blacklist_codes.clone();
            let exclude_lengths = exclude_lengths.clone();
            let base_url = base_url.clone();

            async move {
                let _permit = semaphore.acquire().await.unwrap();

                if let Some(d) = delay {
                    tokio::time::sleep(d).await;
                }

                let result = http_client.check_url(&url, &method).await;
                progress.inc();

                match result {
                    Ok((status, size, redirect)) => {
                        // Check if we should show this result
                        let show = valid_status_codes.contains(&status)
                            && !blacklist_codes.contains(&status)
                            && !exclude_lengths.contains(&size);

                        if show {
                            progress.inc_found();

                            // Extract path from URL
                            let path = url.strip_prefix(&base_url).unwrap_or(&url);

                            // Print to console
                            print_dir_result(
                                path,
                                status,
                                size,
                                redirect.as_deref(),
                                show_length,
                                expanded,
                                &base_url,
                            );

                            // Write to file if configured
                            if let Some(writer) = output.file_writer() {
                                let result = DirResult {
                                    path: path.to_string(),
                                    status,
                                    size,
                                    redirect,
                                };
                                if writer.is_json() {
                                    let _ = writer.write_json(&result).await;
                                } else {
                                    let line =
                                        format!("{} (Status: {}) [Size: {}]", path, status, size);
                                    let _ = writer.write_line(&line).await;
                                }
                            }

                            Some((url, status, size))
                        } else {
                            None
                        }
                    }
                    Err(e) => {
                        progress.inc_error();
                        if verbose {
                            print_error(&format!("{}: {}", url, e), true);
                        }
                        None
                    }
                }
            }
        })
        .buffer_unordered(args.global.threads)
        .collect()
        .await;

    // Check for backup files if requested
    if discover_backup {
        let found_files: Vec<_> = results
            .iter()
            .filter_map(|r| r.as_ref())
            .map(|(url, _, _)| url.clone())
            .collect();

        for file_url in found_files {
            for ext in BACKUP_EXTENSIONS {
                let backup_url = format!("{}{}", file_url, ext);
                if let Ok((status, size, redirect)) =
                    http_client.check_url(&backup_url, &method).await
                {
                    if valid_status_codes.contains(&status) {
                        let path = backup_url.strip_prefix(&base_url).unwrap_or(&backup_url);
                        print_dir_result(
                            path,
                            status,
                            size,
                            redirect.as_deref(),
                            show_length,
                            expanded,
                            &base_url,
                        );
                    }
                }
            }
        }
    }

    progress.finish();
    output.finalize().await?;

    Ok(())
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
