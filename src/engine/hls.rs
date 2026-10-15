//! HLS (HTTP Live Streaming) m3u8 playlist parser and parallel segment download engine.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use url::Url;

use crate::engine::speed::SpeedLimiter;

pub struct HlsDownloader;

impl HlsDownloader {
    /// Download an HLS (.m3u8) stream and assemble segments into target media file.
    pub async fn download(
        playlist_url: &str,
        save_path: &Path,
        pause_signal: Arc<AtomicBool>,
        progress_cb: impl Fn(u64, Option<u64>, f64) + Send + 'static,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        let mut current_url = playlist_url.to_string();
        let resp = client.get(&current_url).send().await?;
        let text = resp.text().await?;

        // 1. Check if Master Playlist
        let media_playlist_text = if text.contains("#EXT-X-STREAM-INF") {
            // Master playlist: find best stream URL
            let mut best_url = None;
            let mut highest_bandwidth = 0u64;
            let mut pending_bw = 0u64;

            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("#EXT-X-STREAM-INF") {
                    if let Some(pos) = trimmed.find("BANDWIDTH=") {
                        let sub = &trimmed[pos + 10..];
                        let bw_str: String = sub.chars().take_while(|c| c.is_ascii_digit()).collect();
                        pending_bw = bw_str.parse::<u64>().unwrap_or(0);
                    }
                } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    if pending_bw >= highest_bandwidth || best_url.is_none() {
                        highest_bandwidth = pending_bw;
                        best_url = Some(trimmed.to_string());
                    }
                }
            }

            if let Some(sub_path) = best_url {
                let base = Url::parse(&current_url)?;
                let resolved = base.join(&sub_path)?;
                current_url = resolved.to_string();
                let child_resp = client.get(&current_url).send().await?;
                child_resp.text().await?
            } else {
                text
            }
        } else {
            text
        };

        // 2. Parse media playlist segments
        let base_url = Url::parse(&current_url)?;
        let mut segment_urls: Vec<String> = Vec::new();

        for line in media_playlist_text.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                if let Ok(resolved) = base_url.join(trimmed) {
                    segment_urls.push(resolved.to_string());
                }
            }
        }

        if segment_urls.is_empty() {
            return Err("No playable segments found in HLS playlist".into());
        }

        // 3. Download segments and append sequentially
        if let Some(parent) = save_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(save_path)
            .await?;

        let total_segments = segment_urls.len();
        let mut total_downloaded = 0u64;
        let mut speed_calc = SpeedLimiter::new();
        let mut last_report = std::time::Instant::now();

        for (idx, seg_url) in segment_urls.iter().enumerate() {
            if pause_signal.load(Ordering::Relaxed) {
                break;
            }

            // Fetch segment data
            match client.get(seg_url).send().await {
                Ok(seg_resp) => {
                    if let Ok(bytes) = seg_resp.bytes().await {
                        file.write_all(&bytes).await?;
                        let n = bytes.len() as u64;
                        total_downloaded += n;
                        speed_calc.record(bytes.len());
                    }
                }
                Err(e) => {
                    eprintln!("[HLS] Failed downloading segment {}/{}: {}", idx + 1, total_segments, e);
                }
            }

            if last_report.elapsed() >= std::time::Duration::from_millis(200) {
                let speed = speed_calc.current_speed_bps();
                // Estimate total size based on average segment size
                let avg_size = total_downloaded / (idx as u64 + 1);
                let estimated_total = avg_size * total_segments as u64;
                progress_cb(total_downloaded, Some(estimated_total), speed);
                last_report = std::time::Instant::now();
            }
        }

        file.flush().await?;
        Ok(total_downloaded)
    }
}
