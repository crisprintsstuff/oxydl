//! Multi-part concurrent download engine with range requests, pause/resume, and speed throttling.

use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use futures_util::StreamExt;
use tokio::sync::Mutex;

use crate::engine::client::{create_http_client, probe_url};
use crate::engine::speed::SpeedTracker;
use crate::engine::state::{ChunkState, ChunkStatus, DownloadMeta};
use crate::util::file::preallocate_file;

#[derive(Debug, Clone)]
pub struct ProgressEvent {
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub speed_bps: f64,
    pub eta_seconds: Option<u64>,
    pub chunks: Vec<ChunkState>,
    pub is_complete: bool,
    pub is_paused: bool,
    pub error: Option<String>,
}

pub struct DownloadOptions {
    pub url: String,
    pub save_path: PathBuf,
    pub num_connections: usize,
    pub speed_limit_bps: Option<u64>,
}

pub struct Downloader {
    options: DownloadOptions,
    stop_signal: Arc<AtomicBool>,
}

impl Downloader {
    pub fn new(options: DownloadOptions) -> (Self, Arc<AtomicBool>) {
        let stop_signal = Arc::new(AtomicBool::new(false));
        (
            Self {
                options,
                stop_signal: stop_signal.clone(),
            },
            stop_signal,
        )
    }

    /// Run the download, periodically sending progress via the callback.
    pub async fn run<F>(&self, mut on_progress: F)
    where
        F: FnMut(ProgressEvent) + Send + 'static,
    {
        let client = match create_http_client() {
            Ok(c) => c,
            Err(e) => {
                on_progress(ProgressEvent {
                    downloaded_bytes: 0,
                    total_bytes: None,
                    speed_bps: 0.0,
                    eta_seconds: None,
                    chunks: Vec::new(),
                    is_complete: false,
                    is_paused: false,
                    error: Some(format!("Failed to create HTTP client: {}", e)),
                });
                return;
            }
        };

        // Probe URL to inspect headers and range capabilities
        let probe = match probe_url(&client, &self.options.url).await {
            Ok(p) => p,
            Err(e) => {
                on_progress(ProgressEvent {
                    downloaded_bytes: 0,
                    total_bytes: None,
                    speed_bps: 0.0,
                    eta_seconds: None,
                    chunks: Vec::new(),
                    is_complete: false,
                    is_paused: false,
                    error: Some(format!("Failed to inspect URL: {}", e)),
                });
                return;
            }
        };

        let total_size = probe.total_bytes;
        let supports_ranges = probe.supports_ranges && total_size.is_some() && total_size.unwrap() >= 512 * 1024;
        let num_parts = if supports_ranges {
            self.options.num_connections.clamp(1, 32)
        } else {
            1
        };

        let target_path = self.options.save_path.clone();

        // Check if existing resume metadata exists
        let mut meta = DownloadMeta::load_from_disk(&target_path);
        let chunks = if let Some(ref mut m) = meta {
            // Validate that cached meta matches current URL and size
            if m.url == self.options.url && m.total_bytes == total_size && !m.chunks.is_empty() {
                m.chunks.clone()
            } else {
                Self::calculate_chunks(total_size, num_parts)
            }
        } else {
            Self::calculate_chunks(total_size, num_parts)
        };

        // Ensure target directory exists
        if let Some(parent) = target_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // If multi-part with known size, preallocate file
        if supports_ranges {
            if let Some(size) = total_size {
                if !target_path.exists() || target_path.metadata().map(|m| m.len()).unwrap_or(0) < size {
                    if let Err(e) = preallocate_file(&target_path, size) {
                        on_progress(ProgressEvent {
                            downloaded_bytes: 0,
                            total_bytes: total_size,
                            speed_bps: 0.0,
                            eta_seconds: None,
                            chunks,
                            is_complete: false,
                            is_paused: false,
                            error: Some(format!("Failed to allocate disk space: {}", e)),
                        });
                        return;
                    }
                }
            }
        }

        let speed_tracker = Arc::new(Mutex::new(SpeedTracker::new(1.5)));
        {
            let mut st = speed_tracker.lock().await;
            st.set_speed_limit(self.options.speed_limit_bps);
        }

        let shared_chunks = Arc::new(Mutex::new(chunks));

        // Spawn chunk workers
        let mut worker_handles = Vec::new();
        let chunk_count = { shared_chunks.lock().await.len() };

        for chunk_idx in 0..chunk_count {
            let client = client.clone();
            let url = self.options.url.clone();
            let target_file = target_path.clone();
            let shared_chunks = shared_chunks.clone();
            let stop_signal = self.stop_signal.clone();
            let speed_tracker = speed_tracker.clone();
            let speed_limit = self.options.speed_limit_bps;

            let handle = tokio::spawn(async move {
                Self::download_chunk_worker(
                    client,
                    url,
                    target_file,
                    chunk_idx,
                    shared_chunks,
                    stop_signal,
                    speed_tracker,
                    supports_ranges,
                    speed_limit,
                )
                .await
            });
            worker_handles.push(handle);
        }

        // Monitoring and progress update loop
        let mut last_save = Instant::now();
        loop {
            tokio::time::sleep(Duration::from_millis(200)).await;

            let chunks_guard = shared_chunks.lock().await;
            let current_chunks = chunks_guard.clone();
            let total_downloaded: u64 = current_chunks.iter().map(|c| c.downloaded).sum();
            drop(chunks_guard);

            let mut st = speed_tracker.lock().await;
            st.record(total_downloaded);
            let current_speed = st.current_speed_bps();
            let eta = st.estimate_eta(total_downloaded, total_size);
            drop(st);

            let all_complete = current_chunks.iter().all(|c| c.status == ChunkStatus::Completed);
            let any_failed = current_chunks.iter().any(|c| c.status == ChunkStatus::Failed);
            let is_stopped = self.stop_signal.load(Ordering::Relaxed);

            // Periodically persist resume metadata
            if last_save.elapsed() >= Duration::from_secs(2) && !all_complete && !is_stopped {
                let meta_to_save = DownloadMeta {
                    url: self.options.url.clone(),
                    file_name: target_path
                        .file_name()
                        .and_then(|f| f.to_str())
                        .unwrap_or("download.bin")
                        .to_string(),
                    total_bytes: total_size,
                    supports_ranges,
                    chunks: current_chunks.clone(),
                };
                let _ = meta_to_save.save_to_disk(&target_path);
                last_save = Instant::now();
            }

            if all_complete {
                // Download finished successfully! Remove metadata file
                DownloadMeta::remove_from_disk(&target_path);
                on_progress(ProgressEvent {
                    downloaded_bytes: total_size.unwrap_or(total_downloaded),
                    total_bytes: total_size,
                    speed_bps: 0.0,
                    eta_seconds: Some(0),
                    chunks: current_chunks,
                    is_complete: true,
                    is_paused: false,
                    error: None,
                });
                break;
            } else if is_stopped {
                // Paused by user! Save metadata for instant resume
                let meta_to_save = DownloadMeta {
                    url: self.options.url.clone(),
                    file_name: target_path
                        .file_name()
                        .and_then(|f| f.to_str())
                        .unwrap_or("download.bin")
                        .to_string(),
                    total_bytes: total_size,
                    supports_ranges,
                    chunks: current_chunks.clone(),
                };
                let _ = meta_to_save.save_to_disk(&target_path);

                on_progress(ProgressEvent {
                    downloaded_bytes: total_downloaded,
                    total_bytes: total_size,
                    speed_bps: 0.0,
                    eta_seconds: None,
                    chunks: current_chunks,
                    is_complete: false,
                    is_paused: true,
                    error: None,
                });
                break;
            } else if any_failed {
                // Save meta so user can retry
                let meta_to_save = DownloadMeta {
                    url: self.options.url.clone(),
                    file_name: target_path
                        .file_name()
                        .and_then(|f| f.to_str())
                        .unwrap_or("download.bin")
                        .to_string(),
                    total_bytes: total_size,
                    supports_ranges,
                    chunks: current_chunks.clone(),
                };
                let _ = meta_to_save.save_to_disk(&target_path);

                on_progress(ProgressEvent {
                    downloaded_bytes: total_downloaded,
                    total_bytes: total_size,
                    speed_bps: 0.0,
                    eta_seconds: None,
                    chunks: current_chunks,
                    is_complete: false,
                    is_paused: false,
                    error: Some("One or more download connections encountered an error".to_string()),
                });
                break;
            } else {
                on_progress(ProgressEvent {
                    downloaded_bytes: total_downloaded,
                    total_bytes: total_size,
                    speed_bps: current_speed,
                    eta_seconds: eta,
                    chunks: current_chunks,
                    is_complete: false,
                    is_paused: false,
                    error: None,
                });
            }
        }

        // Wait for workers to cleanly shutdown
        for handle in worker_handles {
            let _ = handle.await;
        }
    }

    fn calculate_chunks(total_bytes: Option<u64>, num_parts: usize) -> Vec<ChunkState> {
        let Some(total) = total_bytes else {
            return vec![ChunkState::new(0, 0, 0)];
        };

        if num_parts <= 1 || total <= 512 * 1024 {
            return vec![ChunkState::new(0, 0, total.saturating_sub(1))];
        }

        let chunk_size = total / (num_parts as u64);
        let mut chunks = Vec::with_capacity(num_parts);

        for i in 0..num_parts {
            let start = (i as u64) * chunk_size;
            let end = if i == num_parts - 1 {
                total - 1
            } else {
                ((i + 1) as u64) * chunk_size - 1
            };
            chunks.push(ChunkState::new(i, start, end));
        }

        chunks
    }

    #[allow(clippy::too_many_arguments)]
    async fn download_chunk_worker(
        client: reqwest::Client,
        url: String,
        target_file: PathBuf,
        chunk_idx: usize,
        shared_chunks: Arc<Mutex<Vec<ChunkState>>>,
        stop_signal: Arc<AtomicBool>,
        _speed_tracker: Arc<Mutex<SpeedTracker>>,
        supports_ranges: bool,
        speed_limit_bps: Option<u64>,
    ) {
        // Read initial state of this chunk
        let (start_byte, end_byte, mut downloaded) = {
            let mut chunks = shared_chunks.lock().await;
            let chunk = &mut chunks[chunk_idx];
            if chunk.status == ChunkStatus::Completed {
                return;
            }
            chunk.status = ChunkStatus::Downloading;
            (chunk.start, chunk.end, chunk.downloaded)
        };

        let chunk_req_start = start_byte + downloaded;
        if supports_ranges && chunk_req_start > end_byte {
            let mut chunks = shared_chunks.lock().await;
            chunks[chunk_idx].status = ChunkStatus::Completed;
            return;
        }

        // Prepare HTTP request
        let mut req = client.get(&url);
        if supports_ranges {
            req = req.header(reqwest::header::RANGE, format!("bytes={}-{}", chunk_req_start, end_byte));
        }

        let resp = match req.send().await {
            Ok(r) => {
                if !r.status().is_success() {
                    let mut chunks = shared_chunks.lock().await;
                    chunks[chunk_idx].status = ChunkStatus::Failed;
                    return;
                }
                r
            }
            Err(_) => {
                let mut chunks = shared_chunks.lock().await;
                chunks[chunk_idx].status = ChunkStatus::Failed;
                return;
            }
        };

        // Open local file for seeking and writing
        let mut file = match OpenOptions::new().create(true).write(true).open(&target_file) {
            Ok(f) => f,
            Err(_) => {
                let mut chunks = shared_chunks.lock().await;
                chunks[chunk_idx].status = ChunkStatus::Failed;
                return;
            }
        };

        if supports_ranges {
            if let Err(_) = file.seek(SeekFrom::Start(chunk_req_start)) {
                let mut chunks = shared_chunks.lock().await;
                chunks[chunk_idx].status = ChunkStatus::Failed;
                return;
            }
        } else {
            // If single part with no range support and not resuming, start from beginning
            if downloaded == 0 {
                let _ = file.seek(SeekFrom::Start(0));
            } else {
                let _ = file.seek(SeekFrom::Start(downloaded));
            }
        }

        let mut stream = resp.bytes_stream();
        let chunk_speed_limit = speed_limit_bps.map(|l| (l / (shared_chunks.try_lock().map(|c| c.len()).unwrap_or(1) as u64)).max(1024));

        while let Some(item) = stream.next().await {
            if stop_signal.load(Ordering::Relaxed) {
                let _ = file.flush();
                return;
            }

            let start_write = Instant::now();
            match item {
                Ok(bytes) => {
                    let len = bytes.len() as u64;
                    if let Err(_) = file.write_all(&bytes) {
                        let mut chunks = shared_chunks.lock().await;
                        chunks[chunk_idx].status = ChunkStatus::Failed;
                        return;
                    }
                    downloaded += len;

                    // Update shared state
                    {
                        let mut chunks = shared_chunks.lock().await;
                        chunks[chunk_idx].downloaded = downloaded;
                    }

                    // Throttling if speed limit is set
                    if let Some(limit) = chunk_speed_limit {
                        let elapsed = start_write.elapsed();
                        let allowed_time = Duration::from_secs_f64((len as f64) / (limit as f64));
                        if allowed_time > elapsed {
                            tokio::time::sleep(allowed_time - elapsed).await;
                        }
                    }
                }
                Err(_) => {
                    let mut chunks = shared_chunks.lock().await;
                    chunks[chunk_idx].status = ChunkStatus::Failed;
                    return;
                }
            }
        }

        let _ = file.flush();

        // Mark this chunk as completed
        let mut chunks = shared_chunks.lock().await;
        chunks[chunk_idx].status = ChunkStatus::Completed;
        if !supports_ranges {
            chunks[chunk_idx].end = downloaded.saturating_sub(1);
        }
    }
}
