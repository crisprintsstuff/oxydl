//! Persistence layer for tasks and application settings.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use crate::manager::task::{DownloadTask, TaskStatus};
use crate::util::file::{get_app_data_dir, get_default_download_dir};

fn default_true() -> bool { true }
fn default_false() -> bool { false }
fn default_user_agent() -> String {
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) OxyDL/0.5.0".to_string()
}
fn default_timeout() -> u64 { 30 }
fn default_torrent_port() -> u16 { 6881 }
fn default_torrent_peers() -> usize { 50 }
fn default_ed2k_port() -> u16 { 4662 }
fn default_hls_segments() -> usize { 4 }
fn default_api_port() -> u16 { 19532 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub default_save_dir: PathBuf,
    pub default_connections: usize,
    pub max_concurrent_downloads: usize,
    pub monitor_clipboard: bool,
    pub speed_limit_kbps: Option<u64>,
    pub dark_mode: bool,

    // General & Behavior
    #[serde(default = "default_true")]
    pub auto_start_with_windows: bool,
    #[serde(default = "default_false")]
    pub minimize_to_tray: bool,
    #[serde(default = "default_true")]
    pub play_sound_on_complete: bool,

    // Network & Connection
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
    #[serde(default = "default_timeout")]
    pub socket_timeout_secs: u64,

    // Protocols
    #[serde(default = "default_true")]
    pub ftp_passive_mode: bool,
    #[serde(default = "default_torrent_port")]
    pub torrent_port: u16,
    #[serde(default = "default_torrent_peers")]
    pub torrent_max_peers: usize,
    #[serde(default = "default_ed2k_port")]
    pub ed2k_port: u16,
    #[serde(default = "default_hls_segments")]
    pub hls_parallel_segments: usize,

    // Storage
    #[serde(default = "default_true")]
    pub categorize_subfolders: bool,

    // MCP & Browser Integration
    #[serde(default = "default_true")]
    pub enable_api_server: bool,
    #[serde(default = "default_api_port")]
    pub api_port: u16,
    #[serde(default = "default_true")]
    pub enable_mcp_server: bool,

    // Scheduler
    #[serde(default)]
    pub scheduler: crate::manager::scheduler::SchedulerConfig,

    // Media Converter
    #[serde(default)]
    pub ffmpeg_path: Option<PathBuf>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            default_save_dir: get_default_download_dir(),
            default_connections: 8,
            max_concurrent_downloads: 3,
            monitor_clipboard: true,
            speed_limit_kbps: None,
            dark_mode: true,
            auto_start_with_windows: true,
            minimize_to_tray: false,
            play_sound_on_complete: true,
            user_agent: default_user_agent(),
            socket_timeout_secs: 30,
            ftp_passive_mode: true,
            torrent_port: 6881,
            torrent_max_peers: 50,
            ed2k_port: 4662,
            hls_parallel_segments: 4,
            categorize_subfolders: true,
            enable_api_server: true,
            api_port: 19532,
            enable_mcp_server: true,
            scheduler: crate::manager::scheduler::SchedulerConfig::default(),
            ffmpeg_path: None,
        }
    }
}

pub struct Storage;

impl Storage {
    pub fn tasks_path() -> PathBuf {
        get_app_data_dir().join("tasks.json")
    }

    pub fn settings_path() -> PathBuf {
        get_app_data_dir().join("settings.json")
    }

    pub fn load_tasks() -> Vec<DownloadTask> {
        let path = Self::tasks_path();
        if path.exists() {
            if let Ok(data) = std::fs::read(&path) {
                if let Ok(mut tasks) = serde_json::from_slice::<Vec<DownloadTask>>(&data) {
                    // If app was terminated while downloading, mark active tasks as Paused
                    for task in &mut tasks {
                        if task.status == TaskStatus::Downloading {
                            task.status = TaskStatus::Paused;
                            task.speed_bps = 0.0;
                            task.eta_seconds = None;
                        }
                    }
                    return tasks;
                }
            }
        }
        Vec::new()
    }

    pub fn save_tasks(tasks: &[DownloadTask]) -> std::io::Result<()> {
        let path = Self::tasks_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let data = serde_json::to_vec_pretty(tasks)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, data)
    }

    pub fn load_settings() -> AppSettings {
        let path = Self::settings_path();
        if path.exists() {
            if let Ok(data) = std::fs::read(&path) {
                if let Ok(settings) = serde_json::from_slice::<AppSettings>(&data) {
                    return settings;
                }
            }
        }
        AppSettings::default()
    }

    pub fn save_settings(settings: &AppSettings) -> std::io::Result<()> {
        let path = Self::settings_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let data = serde_json::to_vec_pretty(settings)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, data)
    }
}
