//! State structures for chunks, metadata, and resume files.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChunkStatus {
    Pending,
    Downloading,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ChunkState {
    pub index: usize,
    pub start: u64,
    pub end: u64,
    pub downloaded: u64,
    pub status: ChunkStatus,
}

impl ChunkState {
    pub fn new(index: usize, start: u64, end: u64) -> Self {
        Self {
            index,
            start,
            end,
            downloaded: 0,
            status: ChunkStatus::Pending,
        }
    }

    /// Total bytes this chunk is supposed to download.
    pub fn total_bytes(&self) -> u64 {
        if self.end >= self.start {
            (self.end - self.start) + 1
        } else {
            0
        }
    }

    /// Remaining bytes for this chunk.
    pub fn remaining_bytes(&self) -> u64 {
        self.total_bytes().saturating_sub(self.downloaded)
    }

    /// Fraction complete (0.0 to 1.0).
    pub fn fraction(&self) -> f32 {
        let tot = self.total_bytes();
        if tot > 0 {
            ((self.downloaded as f64) / (tot as f64)).clamp(0.0, 1.0) as f32
        } else {
            0.0
        }
    }
}

/// Metadata saved alongside partially downloaded files to support pause/resume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadMeta {
    pub url: String,
    pub file_name: String,
    pub total_bytes: Option<u64>,
    pub supports_ranges: bool,
    pub chunks: Vec<ChunkState>,
}

impl DownloadMeta {
    pub fn meta_path_for(target_file: &Path) -> std::path::PathBuf {
        let mut p = target_file.as_os_str().to_os_string();
        p.push(".oxydl.meta");
        std::path::PathBuf::from(p)
    }

    pub fn legacy_meta_path_for(target_file: &Path) -> std::path::PathBuf {
        let mut p = target_file.as_os_str().to_os_string();
        p.push(".oxidm.meta");
        std::path::PathBuf::from(p)
    }

    pub fn save_to_disk(&self, target_file: &Path) -> std::io::Result<()> {
        let meta_file = Self::meta_path_for(target_file);
        let data = serde_json::to_vec_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(&meta_file, data)?;
        Ok(())
    }

    pub fn load_from_disk(target_file: &Path) -> Option<Self> {
        let meta_file = Self::meta_path_for(target_file);
        let meta_path = if meta_file.exists() {
            meta_file
        } else {
            let legacy = Self::legacy_meta_path_for(target_file);
            if legacy.exists() {
                legacy
            } else {
                return None;
            }
        };
        let data = std::fs::read(&meta_path).ok()?;
        serde_json::from_slice(&data).ok()
    }

    pub fn remove_from_disk(target_file: &Path) {
        let _ = std::fs::remove_file(Self::meta_path_for(target_file));
        let _ = std::fs::remove_file(Self::legacy_meta_path_for(target_file));
    }
}
