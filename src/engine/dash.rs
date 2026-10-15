//! DASH (Dynamic Adaptive Streaming over HTTP) MPD manifest parser and segment download engine.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use url::Url;

use crate::engine::speed::SpeedLimiter;

pub struct DashDownloader;

impl DashDownloader {
    /// Download a DASH (.mpd) manifest stream and assemble segments into target media file.
    pub async fn download(
        manifest_url: &str,
        save_path: &Path,
        pause_signal: Arc<AtomicBool>,
        progress_cb: impl Fn(u64, Option<u64>, f64) + Send + 'static,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        let resp = client.get(manifest_url).send().await?;
        let xml_text = resp.text().await?;
        let base_url = Url::parse(manifest_url)?;

        // 1. Parse Segment URLs and templates from MPD XML
        let mut segment_urls = Vec::new();

        // Extract initialization segment if specified
        if let Some(init_attr_pos) = xml_text.find("initialization=\"") {
            let sub = &xml_text[init_attr_pos + 16..];
            if let Some(end_quote) = sub.find('"') {
                let init_template = &sub[..end_quote];
                let resolved = init_template.replace("$RepresentationID$", "1");
                if let Ok(url) = base_url.join(&resolved) {
                    segment_urls.push(url.to_string());
                }
            }
        }

        // Extract media template: media="chunk-$Number$.m4s" or media="segment-$Number$.mp4"
        let mut media_template = None;
        let mut start_number = 1u64;

        if let Some(media_pos) = xml_text.find("media=\"") {
            let sub = &xml_text[media_pos + 7..];
            if let Some(end_quote) = sub.find('"') {
                media_template = Some(sub[..end_quote].to_string());
            }
        }

        if let Some(start_pos) = xml_text.find("startNumber=\"") {
            let sub = &xml_text[start_pos + 13..];
            if let Some(end_quote) = sub.find('"') {
                if let Ok(num) = sub[..end_quote].parse::<u64>() {
                    start_number = num;
                }
            }
        }

        // Also extract direct <SegmentURL media="..."/> or <BaseURL>...
        for line in xml_text.lines() {
            let trimmed = line.trim();
            if trimmed.contains("<SegmentURL") && trimmed.contains("media=\"") {
                if let Some(m_pos) = trimmed.find("media=\"") {
                    let sub = &trimmed[m_pos + 7..];
                    if let Some(end_quote) = sub.find('"') {
                        if let Ok(url) = base_url.join(&sub[..end_quote]) {
                            segment_urls.push(url.to_string());
                        }
                    }
                }
            }
        }

        // If template was found, populate segments (sample range of segments)
        if let Some(tmpl) = media_template {
            let max_segments = 20; // Default segment lookahead if unbounded
            for num in start_number..(start_number + max_segments) {
                let segment_path = tmpl
                    .replace("$RepresentationID$", "1")
                    .replace("$Number$", &num.to_string())
                    .replace("$Number%05d$", &format!("{:05}", num));

                if let Ok(url) = base_url.join(&segment_path) {
                    segment_urls.push(url.to_string());
                }
            }
        }

        if segment_urls.is_empty() {
            return Err("No playable representations or segments found in DASH manifest".into());
        }

        // 2. Open output file and write segments
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

            match client.get(seg_url).send().await {
                Ok(seg_resp) => {
                    if seg_resp.status().is_success() {
                        if let Ok(bytes) = seg_resp.bytes().await {
                            file.write_all(&bytes).await?;
                            let n = bytes.len() as u64;
                            total_downloaded += n;
                            speed_calc.record(bytes.len());
                        }
                    } else if seg_resp.status() == reqwest::StatusCode::NOT_FOUND {
                        // Reached end of dynamic stream template
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("[DASH] Failed segment {}/{}: {}", idx + 1, total_segments, e);
                }
            }

            if last_report.elapsed() >= std::time::Duration::from_millis(200) {
                let speed = speed_calc.current_speed_bps();
                let avg_size = total_downloaded / (idx as u64 + 1).max(1);
                let estimated_total = avg_size * total_segments as u64;
                progress_cb(total_downloaded, Some(estimated_total), speed);
                last_report = std::time::Instant::now();
            }
        }

        file.flush().await?;
        Ok(total_downloaded)
    }
}
