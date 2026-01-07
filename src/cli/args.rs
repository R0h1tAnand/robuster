//! CLI argument definitions using clap derive

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "rbuster",
    version = "1.0.0",
    about = "Blazingly fast directory/DNS/vhost buster written in Rust",
    long_about = None,
    propagate_version = true,
    after_help = "AUTHOR:\n   Rohit (@R0h1tAnand)"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Directory/file enumeration mode
    Dir(DirArgs),
    /// DNS subdomain enumeration mode
    Dns(DnsArgs),
    /// Virtual host enumeration mode
    Vhost(VhostArgs),
    /// Fuzzing mode (replaces FUZZ keyword)
    Fuzz(FuzzArgs),
    /// AWS S3 bucket enumeration mode
    S3(S3Args),
    /// Google Cloud Storage enumeration mode
    Gcs(GcsArgs),
    /// TFTP enumeration mode
    Tftp(TftpArgs),
}

/// Global options shared across all modes
#[derive(Args, Debug, Clone)]
pub struct GlobalOpts {
    /// Path to wordlist file
    #[arg(short, long, value_name = "FILE")]
    pub wordlist: PathBuf,

    /// Number of concurrent threads
    #[arg(short, long, default_value = "10", value_name = "N")]
    pub threads: usize,

    /// Output file for results
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Suppress banner and non-essential output
    #[arg(short, long)]
    pub quiet: bool,

    /// Verbose output (show errors)
    #[arg(short, long)]
    pub verbose: bool,

    /// Disable progress bar
    #[arg(short = 'z', long)]
    pub no_progress: bool,

    /// Delay between requests in milliseconds
    #[arg(long, value_name = "MS")]
    pub delay: Option<u64>,

    /// No color output
    #[arg(long)]
    pub no_color: bool,
}

/// HTTP options shared across HTTP-based modes
#[derive(Args, Debug, Clone)]
pub struct HttpOpts {
    /// Custom headers (can be used multiple times)
    #[arg(short = 'H', long = "header", value_name = "HEADER")]
    pub headers: Vec<String>,

    /// Cookies for requests
    #[arg(short, long, value_name = "COOKIE")]
    pub cookies: Option<String>,

    /// Custom User-Agent
    #[arg(short = 'a', long, default_value = "rbuster/1.0", value_name = "UA")]
    pub user_agent: String,

    /// Skip TLS certificate verification
    #[arg(short = 'k', long)]
    pub insecure: bool,

    /// Proxy URL (http://host:port or socks5://host:port)
    #[arg(short, long, value_name = "URL")]
    pub proxy: Option<String>,

    /// HTTP Basic Auth username
    #[arg(short = 'U', long, value_name = "USER")]
    pub username: Option<String>,

    /// HTTP Basic Auth password
    #[arg(short = 'P', long, value_name = "PASS")]
    pub password: Option<String>,

    /// Request timeout in seconds
    #[arg(long, default_value = "10", value_name = "SECS")]
    pub timeout: u64,

    /// Follow redirects
    #[arg(short = 'r', long)]
    pub follow_redirect: bool,

    /// HTTP method to use
    #[arg(long, default_value = "GET", value_name = "METHOD")]
    pub method: String,
}

/// Directory enumeration mode arguments
#[derive(Args, Debug)]
pub struct DirArgs {
    #[command(flatten)]
    pub global: GlobalOpts,

    #[command(flatten)]
    pub http: HttpOpts,

    /// Target URL
    #[arg(short, long, value_name = "URL")]
    pub url: String,

    /// File extensions to search (comma-separated, e.g., php,html,js)
    #[arg(short = 'x', long, value_name = "EXT")]
    pub extensions: Option<String>,

    /// Positive status codes (comma-separated)
    #[arg(
        short = 's',
        long,
        default_value = "200,204,301,302,307,308,401,403,405",
        value_name = "CODES"
    )]
    pub status_codes: String,

    /// Negative status codes to exclude (comma-separated)
    #[arg(short = 'b', long, value_name = "CODES")]
    pub status_codes_blacklist: Option<String>,

    /// Append / to each request
    #[arg(short = 'f', long)]
    pub add_slash: bool,

    /// Print full URLs
    #[arg(short = 'e', long)]
    pub expanded: bool,

    /// Show response length
    #[arg(short = 'l', long)]
    pub show_length: bool,

    /// Exclude responses with specified lengths (comma-separated)
    #[arg(long, value_name = "LENGTHS")]
    pub exclude_length: Option<String>,

    /// Search for backup files when a file is found
    #[arg(long)]
    pub discover_backup: bool,

    /// Force continued operation on wildcard responses
    #[arg(long)]
    pub wildcard: bool,
}

/// DNS subdomain enumeration mode arguments
#[derive(Args, Debug)]
pub struct DnsArgs {
    #[command(flatten)]
    pub global: GlobalOpts,

    /// Target domain
    #[arg(short, long, value_name = "DOMAIN")]
    pub domain: String,

    /// Custom DNS resolver (IP:port)
    #[arg(short = 'r', long, value_name = "RESOLVER")]
    pub resolver: Option<String>,

    /// Show resolved IP addresses
    #[arg(short = 'i', long)]
    pub show_ips: bool,

    /// Show CNAME records
    #[arg(short = 'c', long)]
    pub show_cname: bool,

    /// Force continue on wildcard
    #[arg(long)]
    pub wildcard: bool,

    /// Request timeout in seconds
    #[arg(long, default_value = "5", value_name = "SECS")]
    pub timeout: u64,
}

/// Virtual host enumeration mode arguments
#[derive(Args, Debug)]
pub struct VhostArgs {
    #[command(flatten)]
    pub global: GlobalOpts,

    #[command(flatten)]
    pub http: HttpOpts,

    /// Target URL (use IP address)
    #[arg(short, long, value_name = "URL")]
    pub url: String,

    /// Append base domain to wordlist entries
    #[arg(long)]
    pub append_domain: bool,

    /// Domain to append (required if --append-domain is set)
    #[arg(long, value_name = "DOMAIN")]
    pub domain: Option<String>,

    /// Exclude responses with specified length
    #[arg(long, value_name = "LENGTH")]
    pub exclude_length: Option<String>,
}

/// Fuzzing mode arguments
#[derive(Args, Debug)]
pub struct FuzzArgs {
    #[command(flatten)]
    pub global: GlobalOpts,

    #[command(flatten)]
    pub http: HttpOpts,

    /// Target URL with FUZZ keyword
    #[arg(short, long, value_name = "URL")]
    pub url: String,

    /// POST data with FUZZ keyword
    #[arg(short = 'd', long, value_name = "DATA")]
    pub data: Option<String>,

    /// Exclude responses with specified status codes
    #[arg(long, value_name = "CODES")]
    pub exclude_status: Option<String>,

    /// Exclude responses with specified length
    #[arg(long, value_name = "LENGTH")]
    pub exclude_length: Option<String>,

    /// Filter responses containing this string
    #[arg(long, value_name = "STRING")]
    pub filter_string: Option<String>,
}

/// AWS S3 bucket enumeration mode arguments
#[derive(Args, Debug)]
pub struct S3Args {
    #[command(flatten)]
    pub global: GlobalOpts,

    /// Max files to list per bucket
    #[arg(long, default_value = "5", value_name = "N")]
    pub max_files: usize,

    /// Request timeout in seconds
    #[arg(long, default_value = "10", value_name = "SECS")]
    pub timeout: u64,
}

/// Google Cloud Storage enumeration mode arguments
#[derive(Args, Debug)]
pub struct GcsArgs {
    #[command(flatten)]
    pub global: GlobalOpts,

    /// Max files to list per bucket
    #[arg(long, default_value = "5", value_name = "N")]
    pub max_files: usize,

    /// Request timeout in seconds
    #[arg(long, default_value = "10", value_name = "SECS")]
    pub timeout: u64,
}

/// TFTP enumeration mode arguments
#[derive(Args, Debug)]
pub struct TftpArgs {
    #[command(flatten)]
    pub global: GlobalOpts,

    /// TFTP server address
    #[arg(short = 's', long, value_name = "SERVER")]
    pub server: String,

    /// Request timeout in seconds
    #[arg(long, default_value = "5", value_name = "SECS")]
    pub timeout: u64,
}

// Helper functions for parsing comma-separated values
impl DirArgs {
    pub fn parse_status_codes(&self) -> Vec<u16> {
        self.status_codes
            .split(',')
            .filter_map(|s| s.trim().parse::<u16>().ok())
            .collect()
    }

    pub fn parse_status_codes_blacklist(&self) -> Vec<u16> {
        self.status_codes_blacklist
            .as_ref()
            .map(|s| {
                s.split(',')
                    .filter_map(|c| c.trim().parse::<u16>().ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn parse_extensions(&self) -> Vec<String> {
        self.extensions
            .as_ref()
            .map(|s| s.split(',').map(|e| e.trim().to_string()).collect())
            .unwrap_or_default()
    }

    pub fn parse_exclude_lengths(&self) -> Vec<usize> {
        self.exclude_length
            .as_ref()
            .map(|s| s.split(',').filter_map(|l| l.trim().parse().ok()).collect())
            .unwrap_or_default()
    }
}
