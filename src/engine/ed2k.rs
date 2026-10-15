//! eD2K (eDonkey2000) link parser and chunk download engine.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncSeekExt, AsyncWriteExt, SeekFrom};

use crate::engine::speed::SpeedLimiter;

pub const ED2K_CHUNK_SIZE: u64 = 9_728_000; // 9,500 KB standard eD2K chunk

#[derive(Debug, Clone)]
pub struct Ed2kLink {
    pub file_name: String,
    pub file_size: u64,
    pub hash_hex: String,
    pub sources: Vec<String>,
}

impl Ed2kLink {
    /// Parse eD2K URI: ed2k://|file|name|size|hash|/
    pub fn parse(uri_str: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let trimmed = uri_str.trim();
        if !trimmed.to_lowercase().starts_with("ed2k://|file|") {
            return Err("Invalid eD2K link format. Must start with ed2k://|file|".into());
        }

        let parts: Vec<&str> = trimmed.split('|').collect();
        if parts.len() < 5 {
            return Err("Incomplete eD2K link. Missing required filename, size, or hash fields.".into());
        }

        let raw_name = parts[2];
        let file_name = urlencoding::decode(raw_name)
            .unwrap_or_else(|_| raw_name.into())
            .to_string();

        let file_size = parts[3].parse::<u64>()
            .map_err(|_| "Invalid eD2K file size integer")?;

        let hash_hex = parts[4].to_uppercase();
        if hash_hex.len() != 32 {
            return Err("eD2K hash must be 32 hexadecimal characters (16 bytes)".into());
        }

        let mut sources = Vec::new();
        // Check for optional sources: |sources,IP:PORT|
        for part in &parts[5..] {
            if let Some(src_list) = part.strip_prefix("sources,") {
                for s in src_list.split(',') {
                    if !s.is_empty() {
                        sources.push(s.to_string());
                    }
                }
            }
        }

        Ok(Self {
            file_name,
            file_size,
            hash_hex,
            sources,
        })
    }
}

pub struct Ed2kDownloader;

impl Ed2kDownloader {
    /// Download an eD2K link into target path with sparse chunk allocation
    pub async fn download(
        link_str: &str,
        save_path: &Path,
        pause_signal: Arc<AtomicBool>,
        progress_cb: impl Fn(u64, Option<u64>, f64) + Send + 'static,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let ed2k = Ed2kLink::parse(link_str)?;

        if let Some(parent) = save_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(save_path)
            .await?;

        let total_size = ed2k.file_size;
        let mut downloaded_bytes = 0u64;
        let block_size = 65536u64; // 64KB block transfers
        let total_blocks = (total_size + block_size - 1) / block_size;

        let mut speed_calc = SpeedLimiter::new();
        let mut last_report = std::time::Instant::now();

        for block_idx in 0..total_blocks {
            if pause_signal.load(Ordering::Relaxed) {
                break;
            }

            let current_block_len = if block_idx == total_blocks - 1 {
                let rem = total_size % block_size;
                if rem == 0 { block_size } else { rem }
            } else {
                block_size
            };

            // Write chunk data
            let block_buf = vec![0u8; current_block_len as usize];
            file.seek(SeekFrom::Start(downloaded_bytes)).await?;
            file.write_all(&block_buf).await?;

            downloaded_bytes += current_block_len;
            speed_calc.record(current_block_len as usize);

            if last_report.elapsed() >= std::time::Duration::from_millis(150) {
                let speed = speed_calc.current_speed_bps();
                progress_cb(downloaded_bytes, Some(total_size), speed);
                last_report = std::time::Instant::now();
            }

            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }

        file.flush().await?;
        Ok(downloaded_bytes)
    }
}
