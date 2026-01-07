//! TFTP file enumeration mode

use crate::cli::TftpArgs;
use crate::core::load_wordlist;
use crate::error::Result;
use crate::output::{print_error, OutputHandler, ProgressTracker};
use colored::*;
use futures::stream::{self, StreamExt};
use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

// TFTP opcodes
const TFTP_RRQ: u16 = 1; // Read request
const TFTP_DATA: u8 = 3;
const TFTP_ERROR: u8 = 5;
const TFTP_OACK: u8 = 6;

/// Run TFTP file enumeration
pub async fn run(args: TftpArgs) -> Result<()> {
    // Parse server address
    let server_addr: SocketAddr = if args.server.contains(':') {
        args.server.parse().map_err(|e| {
            crate::error::RbusterError::ConfigError(format!("Invalid server address: {}", e))
        })?
    } else {
        format!("{}:69", args.server).parse().map_err(|e| {
            crate::error::RbusterError::ConfigError(format!("Invalid server address: {}", e))
        })?
    };

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
    // TFTP uses UDP, so we limit concurrency more strictly
    let semaphore = Arc::new(Semaphore::new(args.global.threads.min(50)));
    let delay = args.global.delay.map(Duration::from_millis);
    let timeout = Duration::from_secs(args.timeout);
    let verbose = args.global.verbose;

    // Process filenames concurrently
    let _results: Vec<_> = stream::iter(wordlist)
        .map(|filename| {
            let semaphore = Arc::clone(&semaphore);
            let progress = progress.clone();
            let output = Arc::clone(&output);

            async move {
                let _permit = semaphore.acquire().await.unwrap();

                if let Some(d) = delay {
                    tokio::time::sleep(d).await;
                }

                progress.inc();

                // Check if file exists via TFTP
                match check_tftp_file(&server_addr, &filename, timeout).await {
                    Ok(true) => {
                        progress.inc_found();

                        // Print found file
                        println!("{} {}", "Found:".bright_green(), filename.bright_white());

                        // Write to file if configured
                        if let Some(writer) = output.file_writer() {
                            let _ = writer.write_line(&filename).await;
                        }

                        Some(filename)
                    }
                    Ok(false) => None,
                    Err(e) => {
                        if verbose {
                            print_error(&format!("{}: {}", filename, e), true);
                        }
                        None
                    }
                }
            }
        })
        .buffer_unordered(args.global.threads.min(50))
        .collect()
        .await;

    progress.finish();
    output.finalize().await?;

    Ok(())
}

/// Check if a file exists on a TFTP server
async fn check_tftp_file(
    server: &SocketAddr,
    filename: &str,
    timeout: Duration,
) -> std::result::Result<bool, String> {
    // Create UDP socket
    let socket =
        UdpSocket::bind("0.0.0.0:0").map_err(|e| format!("Failed to bind socket: {}", e))?;

    socket
        .set_read_timeout(Some(timeout))
        .map_err(|e| format!("Failed to set timeout: {}", e))?;

    socket
        .set_write_timeout(Some(timeout))
        .map_err(|e| format!("Failed to set timeout: {}", e))?;

    // Build TFTP read request packet
    // Format: opcode (2 bytes) | filename | 0 | mode | 0 | blksize | 0 | 512 | 0
    let mut packet = Vec::new();
    packet.extend_from_slice(&TFTP_RRQ.to_be_bytes());
    packet.extend_from_slice(filename.as_bytes());
    packet.push(0);
    packet.extend_from_slice(b"octet");
    packet.push(0);
    packet.extend_from_slice(b"blksize");
    packet.push(0);
    packet.extend_from_slice(b"512");
    packet.push(0);

    // Send request
    socket
        .send_to(&packet, server)
        .map_err(|e| format!("Failed to send request: {}", e))?;

    // Receive response
    let mut buf = [0u8; 516];
    match socket.recv_from(&mut buf) {
        Ok((size, _)) if size >= 4 => {
            let opcode = buf[1];
            match opcode {
                TFTP_DATA | TFTP_OACK => Ok(true), // File exists
                TFTP_ERROR => Ok(false),           // File not found or access denied
                _ => Ok(false),
            }
        }
        Ok(_) => Ok(false),
        Err(e)
            if e.kind() == std::io::ErrorKind::WouldBlock
                || e.kind() == std::io::ErrorKind::TimedOut =>
        {
            Ok(false) // Timeout, assume file doesn't exist
        }
        Err(e) => Err(format!("Failed to receive response: {}", e)),
    }
}
