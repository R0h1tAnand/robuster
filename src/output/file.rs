//! File output handlers (text and JSON)

use serde::Serialize;
use std::path::Path;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

/// Result types for JSON output
#[derive(Serialize, Clone)]
pub struct DirResult {
    pub path: String,
    pub status: u16,
    pub size: usize,
    pub redirect: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct DnsResultJson {
    pub subdomain: String,
    pub ips: Vec<String>,
    pub cnames: Vec<String>,
}

#[derive(Serialize, Clone)]
pub struct VhostResult {
    pub host: String,
    pub status: u16,
    pub size: usize,
}

#[derive(Serialize, Clone)]
pub struct FuzzResult {
    pub payload: String,
    pub status: u16,
    pub size: usize,
    pub words: usize,
    pub lines: usize,
}

#[derive(Serialize, Clone)]
pub struct BucketResult {
    pub name: String,
    pub status: String,
    pub files: Vec<String>,
}

/// File writer with buffering
pub struct FileWriter {
    file: Mutex<File>,
    json_mode: bool,
    first_entry: Mutex<bool>,
}

impl FileWriter {
    pub async fn new(path: &Path) -> std::io::Result<Self> {
        let json_mode = path.extension().map(|ext| ext == "json").unwrap_or(false);

        let mut file = File::create(path).await?;

        if json_mode {
            file.write_all(b"[\n").await?;
        }

        Ok(Self {
            file: Mutex::new(file),
            json_mode,
            first_entry: Mutex::new(true),
        })
    }

    pub async fn write_line(&self, line: &str) -> std::io::Result<()> {
        let mut file = self.file.lock().await;
        file.write_all(line.as_bytes()).await?;
        file.write_all(b"\n").await?;
        Ok(())
    }

    pub async fn write_json<T: Serialize>(&self, item: &T) -> std::io::Result<()> {
        let mut file = self.file.lock().await;
        let mut first = self.first_entry.lock().await;

        if !*first {
            file.write_all(b",\n").await?;
        }
        *first = false;

        let json = serde_json::to_string_pretty(item).map_err(std::io::Error::other)?;
        file.write_all(json.as_bytes()).await?;

        Ok(())
    }

    pub async fn finalize(&self) -> std::io::Result<()> {
        if self.json_mode {
            let mut file = self.file.lock().await;
            file.write_all(b"\n]\n").await?;
        }
        Ok(())
    }

    pub fn is_json(&self) -> bool {
        self.json_mode
    }
}

/// Output handler that can write to both console and file
pub struct OutputHandler {
    file_writer: Option<Arc<FileWriter>>,
}

impl OutputHandler {
    pub async fn new(output_path: Option<&Path>) -> std::io::Result<Self> {
        let file_writer = if let Some(path) = output_path {
            Some(Arc::new(FileWriter::new(path).await?))
        } else {
            None
        };

        Ok(Self { file_writer })
    }

    pub fn file_writer(&self) -> Option<Arc<FileWriter>> {
        self.file_writer.clone()
    }

    pub async fn finalize(&self) -> std::io::Result<()> {
        if let Some(ref writer) = self.file_writer {
            writer.finalize().await?;
        }
        Ok(())
    }
}
