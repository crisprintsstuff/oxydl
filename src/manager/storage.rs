//! Persistence layer for tasks and application settings.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use crate::manager::task::{DownloadTask, TaskStatus};
use crate::util::file::{get_app_data_dir, get_default_download_dir};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub default_save_dir: PathBuf,
    pub default_connections: usize,
    pub max_concurrent_downloads: usize,
    pub monitor_clipboard: bool,
    pub speed_limit_kbps: Option<u64>,
    pub dark_mode: bool,
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
