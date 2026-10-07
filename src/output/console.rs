//! Console output with colors

use colored::*;
use std::net::IpAddr;

/// Format a found result for directory mode
pub fn format_dir_result(
    path: &str,
    status: u16,
    size: usize,
    redirect: Option<&str>,
    show_length: bool,
    expanded: bool,
    base_url: &str,
) -> String {
    let status_colored = match status {
        200..=299 => status.to_string().bright_green(),
        300..=399 => status.to_string().bright_yellow(),
        400..=499 => status.to_string().bright_red(),
        _ => status.to_string().white(),
    };

    let display_path = if expanded {
        format!("{}{}", base_url.trim_end_matches('/'), path)
    } else {
        path.to_string()
    };

    let mut line = format!(
        "{:<30} (Status: {})",
        display_path.bright_white(),
        status_colored
    );

    if show_length {
        line.push_str(&format!(" [Size: {}]", size.to_string().bright_cyan()));
    }

    if let Some(loc) = redirect {
        line.push_str(&format!(" [--> {}]", loc.bright_magenta()));
    }

    line
}

/// Format a found result for DNS mode
pub fn format_dns_result(
    subdomain: &str,
    ips: &[IpAddr],
    cnames: &[String],
    show_ips: bool,
    show_cname: bool,
) -> String {
    let mut line = format!("{}", subdomain.bright_green());

    if show_ips && !ips.is_empty() {
        let ip_str: String = ips
            .iter()
            .map(|ip| ip.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        line.push_str(&format!(" [{}]", ip_str.bright_cyan()));
    }

    if show_cname && !cnames.is_empty() {
        let cname_str = cnames.join(", ");
        line.push_str(&format!(" [CNAME: {}]", cname_str.bright_yellow()));
    }

    line
}

/// Format a found result for vhost mode
pub fn format_vhost_result(host: &str, status: u16, size: usize) -> String {
    let status_colored = match status {
        200..=299 => status.to_string().bright_green(),
        300..=399 => status.to_string().bright_yellow(),
        400..=499 => status.to_string().bright_red(),
        _ => status.to_string().white(),
    };

    format!(
        "Found: {} (Status: {}) [Size: {}]",
        host.bright_green(),
        status_colored,
        size.to_string().bright_cyan()
    )
}

/// Format a found result for fuzz mode
pub fn format_fuzz_result(
    payload: &str,
    status: u16,
    size: usize,
    words: usize,
    lines: usize,
) -> String {
    let status_colored = match status {
        200..=299 => status.to_string().bright_green(),
        300..=399 => status.to_string().bright_yellow(),
        400..=499 => status.to_string().bright_red(),
        _ => status.to_string().white(),
    };

    format!(
        "{:<30} [Status: {}, Size: {}, Words: {}, Lines: {}]",
        payload.bright_white(),
        status_colored,
        size.to_string().bright_cyan(),
        words,
        lines
    )
}

/// Format S3/GCS bucket result
pub fn format_bucket_result(bucket: &str, status: &str, files: &[String]) -> String {
    let status_colored = match status {
        "public" => status.bright_green(),
        "private" => status.bright_yellow(),
        "not_found" => status.bright_red(),
        _ => status.white(),
    };

    let mut out = format!("{} [{}]", bucket.bright_white(), status_colored);
    for file in files.iter().take(5) {
        out.push('\n');
        out.push_str(&format!("  └── {}", file.bright_cyan()));
    }
    out
}

/// Print error message (goes to stderr, not affected by progress bar)
pub fn print_error(msg: &str, verbose: bool) {
    if verbose {
        eprintln!("{} {}", "[ERROR]".bright_red(), msg);
    }
}

/// Print warning message (goes to stderr, not affected by progress bar)
pub fn print_warning(msg: &str) {
    eprintln!("{} {}", "[WARN]".bright_yellow(), msg);
}
