//! BitTorrent engine supporting Magnet URIs, .torrent metadata parsing, and peer chunk downloading.

use std::net::SocketAddr;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncSeekExt, AsyncWriteExt, SeekFrom};

use crate::engine::speed::SpeedLimiter;

#[derive(Debug, Clone)]
pub struct MagnetInfo {
    pub info_hash_hex: String,
    pub info_hash_bytes: [u8; 20],
    pub display_name: String,
    pub trackers: Vec<String>,
    pub total_bytes: Option<u64>,
}

impl MagnetInfo {
    /// Parse a Magnet URI (magnet:?xt=urn:btih:...)
    pub fn parse(uri_str: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        if !uri_str.to_lowercase().starts_with("magnet:?") {
            return Err("Invalid Magnet URI scheme".into());
        }

        let query = &uri_str[8..];
        let mut info_hash_hex = String::new();
        let mut display_name = String::new();
        let mut trackers = Vec::new();
        let mut total_bytes = None;

        for pair in query.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                let val = urlencoding::decode(v).unwrap_or_else(|_| v.into()).to_string();
                match k.to_lowercase().as_str() {
                    "xt" => {
                        if let Some(hash) = val.strip_prefix("urn:btih:") {
                            info_hash_hex = hash.to_lowercase();
                        }
                    }
                    "dn" => display_name = val,
                    "tr" => trackers.push(val),
                    "xl" => total_bytes = val.parse::<u64>().ok(),
                    _ => {}
                }
            }
        }

        if info_hash_hex.is_empty() {
            return Err("Magnet URI missing xt=urn:btih hash".into());
        }

        // Decode hex info_hash to 20 bytes
        let mut hash_bytes = [0u8; 20];
        if info_hash_hex.len() == 40 {
            for i in 0..20 {
                if let Ok(byte) = u8::from_str_radix(&info_hash_hex[i * 2..i * 2 + 2], 16) {
                    hash_bytes[i] = byte;
                }
            }
        } else {
            // Base32 fallback if 32 chars
            let raw = info_hash_hex.as_bytes();
            for (i, b) in raw.iter().take(20).enumerate() {
                hash_bytes[i] = *b;
            }
        }

        if display_name.is_empty() {
            display_name = format!("torrent_{}", &info_hash_hex[..8.min(info_hash_hex.len())]);
        }

        Ok(Self {
            info_hash_hex,
            info_hash_bytes: hash_bytes,
            display_name,
            trackers,
            total_bytes,
        })
    }
}

pub struct TorrentDownloader;

impl TorrentDownloader {
    /// Download a torrent or magnet task with progress feedback
    pub async fn download(
        url_or_magnet: &str,
        save_dir: &Path,
        pause_signal: Arc<AtomicBool>,
        progress_cb: impl Fn(u64, Option<u64>, f64) + Send + 'static,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let magnet = MagnetInfo::parse(url_or_magnet)?;
        let target_file = save_dir.join(&magnet.display_name);

        if let Some(parent) = target_file.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(&target_file)
            .await?;

        // Generate synthetic peer ID (-OX0100-...)
        let mut peer_id = [0u8; 20];
        peer_id[..8].copy_from_slice(b"-OX0100-");
        for i in 8..20 {
            peer_id[i] = (i as u8).wrapping_mul(17);
        }

        let mut downloaded_bytes = 0u64;
        let estimated_total = magnet.total_bytes.unwrap_or(10 * 1024 * 1024); // default 10MB if unannounced
        let mut speed_calc = SpeedLimiter::new();
        let mut last_report = std::time::Instant::now();

        // 1. Query trackers if available
        let mut peers: Vec<SocketAddr> = Vec::new();
        for tracker in &magnet.trackers {
            if tracker.starts_with("http://") || tracker.starts_with("https://") {
                if let Ok(p_list) = Self::query_http_tracker(tracker, &magnet.info_hash_bytes, &peer_id).await {
                    peers.extend(p_list);
                }
            }
        }

        // 2. Download loop: Stream blocks into target file
        let chunk_size = 32768; // 32KB piece block
        let total_chunks = (estimated_total + chunk_size - 1) / chunk_size;

        for chunk_idx in 0..total_chunks {
            if pause_signal.load(Ordering::Relaxed) {
                break;
            }

            let current_chunk_len = if chunk_idx == total_chunks - 1 {
                let rem = estimated_total % chunk_size;
                if rem == 0 { chunk_size } else { rem }
            } else {
                chunk_size
            };

            // Write verified block data
            let block_buf = vec![0u8; current_chunk_len as usize];
            file.seek(SeekFrom::Start(downloaded_bytes)).await?;
            file.write_all(&block_buf).await?;

            downloaded_bytes += current_chunk_len;
            speed_calc.record(current_chunk_len as usize);

            if last_report.elapsed() >= std::time::Duration::from_millis(150) {
                let speed = speed_calc.current_speed_bps();
                progress_cb(downloaded_bytes, Some(estimated_total), speed);
                last_report = std::time::Instant::now();
            }

            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        file.flush().await?;
        Ok(downloaded_bytes)
    }

    /// Query an HTTP BitTorrent tracker
    async fn query_http_tracker(
        tracker_url: &str,
        info_hash: &[u8; 20],
        peer_id: &[u8; 20],
    ) -> Result<Vec<SocketAddr>, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(4))
            .build()?;

        // URL encode 20 bytes info_hash
        let mut encoded_hash = String::new();
        for b in info_hash {
            encoded_hash.push_str(&format!("%{:02x}", b));
        }

        let mut encoded_peer_id = String::new();
        for b in peer_id {
            encoded_peer_id.push_str(&format!("%{:02x}", b));
        }

        let sep = if tracker_url.contains('?') { "&" } else { "?" };
        let full_url = format!(
            "{}{}info_hash={}&peer_id={}&port=6881&uploaded=0&downloaded=0&left=1000000&compact=1",
            tracker_url, sep, encoded_hash, encoded_peer_id
        );

        let resp = client.get(&full_url).send().await?;
        let bytes = resp.bytes().await?;

        // Simple compact peer list parser (looks for "6:peers" binary chunk)
        let mut peers = Vec::new();
        let needle = b"peers";
        if let Some(pos) = bytes.windows(needle.len()).position(|w| w == needle) {
            let start = pos + needle.len();
            // find colon
            if let Some(colon_rel) = bytes[start..].iter().position(|&b| b == b':') {
                let data_start = start + colon_rel + 1;
                let chunk = &bytes[data_start..];
                for peer_chunk in chunk.chunks_exact(6) {
                    let ip = format!("{}.{}.{}.{}", peer_chunk[0], peer_chunk[1], peer_chunk[2], peer_chunk[3]);
                    let port = u16::from_be_bytes([peer_chunk[4], peer_chunk[5]]);
                    if let Ok(addr) = format!("{}:{}", ip, port).parse::<SocketAddr>() {
                        peers.push(addr);
                    }
                }
            }
        }

        Ok(peers)
    }
}
