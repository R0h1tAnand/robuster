# rbuster

> Blazingly fast directory/DNS/vhost buster written in Rust 🦀

A high-performance alternative to [gobuster](https://github.com/OJ/gobuster), leveraging Rust's async runtime for maximum speed.

## Features

- **Blazingly Fast** - Async I/O with tokio, connection pooling, zero-copy optimizations
- **7 Modes** - dir, dns, vhost, fuzz, s3, gcs, tftp
- **TLS Support** - Full TLS/SSL support via rustls (no OpenSSL required)
- **Proxy Support** - HTTP and SOCKS5 proxy support
- **Beautiful Output** - Colored output with progress bar
- **File Output** - Text and JSON output formats

## Installation

### Build from source

```bash
git clone https://github.com/rohit/rbuster
cd rbuster
cargo build --release
./target/release/rbuster --help
```

## Usage

### Directory Enumeration (dir)

```bash
# Basic usage
rbuster dir -u https://example.com -w wordlist.txt

# With extensions
rbuster dir -u https://example.com -w wordlist.txt -x php,html,js

# Show response length, follow redirects
rbuster dir -u https://example.com -w wordlist.txt -l -r

# With custom headers and cookies
rbuster dir -u https://example.com -w wordlist.txt -H "Authorization: Bearer token" -c "session=abc"

# High thread count for speed
rbuster dir -u https://example.com -w wordlist.txt -t 50
```

### DNS Subdomain Enumeration (dns)

```bash
# Basic usage
rbuster dns -d example.com -w subdomains.txt

# Show IPs and CNAMEs
rbuster dns -d example.com -w subdomains.txt -i -c

# Custom DNS resolver
rbuster dns -d example.com -w subdomains.txt -r 8.8.8.8
```

### Virtual Host Discovery (vhost)

```bash
# Basic usage
rbuster vhost -u https://10.10.10.10 -w vhosts.txt

# Append domain to wordlist entries
rbuster vhost -u https://10.10.10.10 -w vhosts.txt --append-domain --domain example.com
```

### Fuzzing (fuzz)

```bash
# URL parameter fuzzing
rbuster fuzz -u "https://example.com?id=FUZZ" -w payloads.txt

# POST data fuzzing
rbuster fuzz -u https://example.com/login -w passwords.txt -d "user=admin&pass=FUZZ" --method POST

# Header fuzzing
rbuster fuzz -u https://example.com -w tokens.txt -H "X-API-Key: FUZZ"
```

### S3 Bucket Enumeration (s3)

```bash
rbuster s3 -w bucket-names.txt
```

### GCS Bucket Enumeration (gcs)

```bash
rbuster gcs -w bucket-names.txt
```

### TFTP Enumeration (tftp)

```bash
rbuster tftp -s 10.10.10.10 -w filenames.txt
```

## Global Options

| Option | Description |
|--------|-------------|
| `-w, --wordlist` | Path to wordlist file |
| `-t, --threads` | Concurrent threads (default: 10) |
| `-o, --output` | Output file (supports .json) |
| `-q, --quiet` | Suppress banner |
| `-v, --verbose` | Show errors |
| `-z, --no-progress` | Disable progress bar |
| `--delay` | Delay between requests (ms) |

## Performance

rbuster is designed to exceed gobuster's performance through:

- **Tokio async runtime** - Non-blocking concurrent I/O
- **Connection pooling** - Reuses HTTP connections
- **Streaming wordlist** - Memory efficient for large files
- **Release optimizations** - LTO, codegen-units=1, stripped binary

## License

MIT
