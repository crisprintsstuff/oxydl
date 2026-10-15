//! Download task definitions and status types.

use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::engine::state::ChunkState;
use crate::util::file::TaskCategory;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DownloadProtocol {
    Http,
    Ftp,
    Torrent,
    Ed2k,
    Hls,
    Dash,
}

impl DownloadProtocol {
    pub fn detect(url: &str) -> Self {
        let trimmed = url.trim();
        let lower = trimmed.to_lowercase();
        if lower.starts_with("ftp://") {
            Self::Ftp
        } else if lower.starts_with("magnet:") || lower.ends_with(".torrent") || lower.contains(".torrent?") {
            Self::Torrent
        } else if lower.starts_with("ed2k://") {
            Self::Ed2k
        } else if lower.contains(".m3u8") || lower.starts_with("hls://") {
            Self::Hls
        } else if lower.contains(".mpd") || lower.starts_with("dash://") {
            Self::Dash
        } else {
            Self::Http
        }
    }

    pub fn display_badge(&self) -> &'static str {
        match self {
            Self::Http => "HTTP/S",
            Self::Ftp => "FTP",
            Self::Torrent => "TORRENT",
            Self::Ed2k => "eD2K",
            Self::Hls => "HLS",
            Self::Dash => "DASH",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Http => "🌐",
            Self::Ftp => "📁",
            Self::Torrent => "🧲",
            Self::Ed2k => "🐴",
            Self::Hls => "📺",
            Self::Dash => "🎬",
        }
    }

    pub fn rgb(&self) -> (u8, u8, u8) {
        match self {
            Self::Http => (52, 152, 219),    // Blue
            Self::Ftp => (230, 126, 34),     // Orange
            Self::Torrent => (155, 89, 182), // Purple
            Self::Ed2k => (26, 188, 156),    // Teal
            Self::Hls => (46, 204, 113),     // Green
            Self::Dash => (241, 196, 15),    // Amber
        }
    }
}

fn default_protocol() -> DownloadProtocol {
    DownloadProtocol::Http
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum TaskStatus {
    Queued,
    Downloading,
    Paused,
    Completed,
    Failed(String),
}

impl TaskStatus {
    pub fn display_name(&self) -> String {
        match self {
            TaskStatus::Queued => "Queued".to_string(),
            TaskStatus::Downloading => "Downloading".to_string(),
            TaskStatus::Paused => "Paused".to_string(),
            TaskStatus::Completed => "Completed".to_string(),
            TaskStatus::Failed(err) => format!("Error: {}", err),
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, TaskStatus::Downloading)
    }

    pub fn is_finished(&self) -> bool {
        matches!(self, TaskStatus::Completed)
    }

    pub fn can_resume(&self) -> bool {
        matches!(self, TaskStatus::Paused | TaskStatus::Failed(_) | TaskStatus::Queued)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadTask {
    pub id: Uuid,
    #[serde(default = "default_protocol")]
    pub protocol: DownloadProtocol,
    pub url: String,
    pub file_name: String,
    pub save_path: PathBuf,
    pub category: TaskCategory,
    pub status: TaskStatus,
    pub total_bytes: Option<u64>,
    pub downloaded_bytes: u64,
    pub speed_bps: f64,
    pub eta_seconds: Option<u64>,
    pub num_connections: usize,
    pub chunks: Vec<ChunkState>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

impl DownloadTask {
    pub fn new(
        url: String,
        file_name: String,
        save_path: PathBuf,
        num_connections: usize,
        category: TaskCategory,
    ) -> Self {
        let protocol = DownloadProtocol::detect(&url);
        Self {
            id: Uuid::new_v4(),
            protocol,
            url,
            file_name,
            save_path,
            category,
            status: TaskStatus::Queued,
            total_bytes: None,
            downloaded_bytes: 0,
            speed_bps: 0.0,
            eta_seconds: None,
            num_connections,
            chunks: Vec::new(),
            created_at: Utc::now(),
            completed_at: None,
            error_message: None,
        }
    }

    /// Progress percentage fraction (0.0 to 1.0).
    pub fn fraction(&self) -> f32 {
        if self.status == TaskStatus::Completed {
            return 1.0;
        }
        match self.total_bytes {
            Some(tot) if tot > 0 => ((self.downloaded_bytes as f64) / (tot as f64)).clamp(0.0, 1.0) as f32,
            _ => 0.0,
        }
    }
}
