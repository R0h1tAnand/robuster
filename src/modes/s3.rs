//! AWS S3 bucket enumeration mode

use crate::cli::S3Args;
use crate::core::load_wordlist;
use crate::error::Result;
use crate::output::{
    print_bucket_result, print_error, BucketResult, OutputHandler, ProgressTracker,
};
use futures::stream::{self, StreamExt};
use reqwest::{Client, ClientBuilder, StatusCode};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

#[allow(dead_code)]
const S3_REGIONS: &[&str] = &[
    "us-east-1",
    "us-east-2",
    "us-west-1",
    "us-west-2",
    "eu-west-1",
    "eu-west-2",
    "eu-west-3",
    "eu-central-1",
    "ap-southeast-1",
    "ap-southeast-2",
    "ap-northeast-1",
];

/// Run S3 bucket enumeration
pub async fn run(args: S3Args) -> Result<()> {
    // Build HTTP client
    let client = Arc::new(
        ClientBuilder::new()
            .user_agent("rbuster/1.0")
            .timeout(Duration::from_secs(args.timeout))
            .pool_max_idle_per_host(50)
            .tcp_nodelay(true)
            .build()?,
    );

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
    let max_files = args.max_files;
    let verbose = args.global.verbose;

    // Process bucket names concurrently
    let _results: Vec<_> = stream::iter(wordlist)
        .map(|bucket_name| {
            let semaphore = Arc::clone(&semaphore);
            let client = Arc::clone(&client);
            let progress = progress.clone();
            let output = Arc::clone(&output);

            async move {
                let _permit = semaphore.acquire().await.unwrap();

                if let Some(d) = delay {
                    tokio::time::sleep(d).await;
                }

                progress.inc();

                // Try different S3 URL formats
                let urls = vec![
                    format!("https://{}.s3.amazonaws.com", bucket_name),
                    format!("https://s3.amazonaws.com/{}", bucket_name),
                ];

                for url in urls {
                    match check_s3_bucket(&client, &url, max_files).await {
                        Ok(Some((status, files))) => {
                            progress.inc_found();

                            print_bucket_result(&bucket_name, &status, &files);

                            // Write to file if configured
                            if let Some(writer) = output.file_writer() {
                                let result = BucketResult {
                                    name: bucket_name.clone(),
                                    status: status.clone(),
                                    files: files.clone(),
                                };
                                if writer.is_json() {
                                    let _ = writer.write_json(&result).await;
                                } else {
                                    let line = format!(
                                        "{} [{}] files: {}",
                                        bucket_name,
                                        status,
                                        files.len()
                                    );
                                    let _ = writer.write_line(&line).await;
                                }
                            }

                            return Some((bucket_name, status, files));
                        }
                        Ok(None) => continue,
                        Err(e) => {
                            if verbose {
                                print_error(&format!("{}: {}", bucket_name, e), true);
                            }
                            continue;
                        }
                    }
                }

                None
            }
        })
        .buffer_unordered(args.global.threads)
        .collect()
        .await;

    progress.finish();
    output.finalize().await?;

    Ok(())
}

/// Check if an S3 bucket exists and get its status
async fn check_s3_bucket(
    client: &Client,
    url: &str,
    max_files: usize,
) -> std::result::Result<Option<(String, Vec<String>)>, reqwest::Error> {
    let response = client.get(url).send().await?;

    match response.status() {
        StatusCode::OK => {
            // Bucket is public, try to list files
            let body = response.text().await.unwrap_or_default();
            let files = parse_s3_listing(&body, max_files);
            Ok(Some(("public".to_string(), files)))
        }
        StatusCode::FORBIDDEN => {
            // Bucket exists but is private
            Ok(Some(("private".to_string(), vec![])))
        }
        StatusCode::NOT_FOUND => Ok(None),
        _ => Ok(None),
    }
}

/// Parse S3 bucket listing XML to extract file keys
fn parse_s3_listing(xml: &str, max_files: usize) -> Vec<String> {
    let mut files = Vec::new();

    // Simple XML parsing for <Key> elements
    for line in xml.lines() {
        if let Some(start) = line.find("<Key>") {
            if let Some(end) = line.find("</Key>") {
                let key = &line[start + 5..end];
                files.push(key.to_string());
                if files.len() >= max_files {
                    break;
                }
            }
        }
    }

    files
}
