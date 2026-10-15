//! Queue manager coordinating concurrent downloads, task lifecycle, and auto-scheduling.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::engine::downloader::{DownloadOptions, Downloader};
use crate::engine::state::DownloadMeta;
use crate::manager::storage::{AppSettings, Storage};
use crate::manager::task::{DownloadTask, TaskStatus};
use crate::util::file::TaskCategory;

#[derive(Clone)]
pub struct QueueManager {
    pub tasks: Arc<Mutex<Vec<DownloadTask>>>,
    pub settings: Arc<Mutex<AppSettings>>,
    active_controllers: Arc<Mutex<HashMap<Uuid, Arc<AtomicBool>>>>,
}

impl QueueManager {
    pub fn new() -> Self {
        let tasks = Storage::load_tasks();
        let settings = Storage::load_settings();

        Self {
            tasks: Arc::new(Mutex::new(tasks)),
            settings: Arc::new(Mutex::new(settings)),
            active_controllers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Add a new download task. If start_now is true, triggers immediate download.
    pub async fn add_task(
        &self,
        url: String,
        file_name: String,
        save_dir: PathBuf,
        num_connections: usize,
        start_now: bool,
    ) -> Uuid {
        let category = TaskCategory::from_filename(&file_name);
        let save_path = save_dir.join(&file_name);
        let mut task = DownloadTask::new(url, file_name, save_path, num_connections, category);

        if !start_now {
            task.status = TaskStatus::Paused;
        }

        let task_id = task.id;
        {
            let mut tasks = self.tasks.lock().await;
            tasks.insert(0, task);
            let _ = Storage::save_tasks(&tasks);
        }

        if start_now {
            self.start_task(task_id).await;
        }

        task_id
    }

    /// Start or resume a download task by ID.
    pub async fn start_task(&self, task_id: Uuid) {
        let mut controllers = self.active_controllers.lock().await;
        if controllers.contains_key(&task_id) {
            return; // Already downloading
        }

        let mut tasks = self.tasks.lock().await;
        let task = match tasks.iter_mut().find(|t| t.id == task_id) {
            Some(t) => t,
            None => return,
        };

        task.status = TaskStatus::Downloading;
        task.error_message = None;

        let options = DownloadOptions {
            url: task.url.clone(),
            save_path: task.save_path.clone(),
            num_connections: task.num_connections,
            speed_limit_bps: self.settings.lock().await.speed_limit_kbps.map(|kb| kb * 1024),
        };

        let (downloader, stop_signal) = Downloader::new(options);
        controllers.insert(task_id, stop_signal.clone());
        drop(tasks);
        drop(controllers);

        // Spawn background task runner
        let shared_tasks = self.tasks.clone();
        let shared_controllers = self.active_controllers.clone();
        let queue_clone = self.clone();

        tokio::spawn(async move {
            downloader
                .run(move |event| {
                    let shared_tasks = shared_tasks.clone();
                    tokio::spawn(async move {
                        let mut tasks = shared_tasks.lock().await;
                        if let Some(t) = tasks.iter_mut().find(|t| t.id == task_id) {
                            t.downloaded_bytes = event.downloaded_bytes;
                            if let Some(total) = event.total_bytes {
                                t.total_bytes = Some(total);
                            }
                            t.speed_bps = event.speed_bps;
                            t.eta_seconds = event.eta_seconds;
                            t.chunks = event.chunks;

                            if event.is_complete {
                                t.status = TaskStatus::Completed;
                                t.completed_at = Some(chrono::Utc::now());
                                t.speed_bps = 0.0;
                                t.eta_seconds = Some(0);
                            } else if event.is_paused {
                                t.status = TaskStatus::Paused;
                                t.speed_bps = 0.0;
                                t.eta_seconds = None;
                            } else if let Some(err) = event.error {
                                t.status = TaskStatus::Failed(err.clone());
                                t.error_message = Some(err);
                                t.speed_bps = 0.0;
                                t.eta_seconds = None;
                            }
                        }
                    });
                })
                .await;

            // Remove controller when download loop completes
            {
                let mut controllers = shared_controllers.lock().await;
                controllers.remove(&task_id);
            }

            // Save tasks snapshot
            {
                let tasks = queue_clone.tasks.lock().await;
                let _ = Storage::save_tasks(&tasks);
            }

            // Auto schedule next in queue
            queue_clone.schedule_next();
        });
    }

    /// Pause an ongoing download.
    pub async fn pause_task(&self, task_id: Uuid) {
        let mut controllers = self.active_controllers.lock().await;
        if let Some(signal) = controllers.remove(&task_id) {
            signal.store(true, Ordering::Relaxed);
        }

        let mut tasks = self.tasks.lock().await;
        if let Some(t) = tasks.iter_mut().find(|t| t.id == task_id) {
            if t.status == TaskStatus::Downloading || t.status == TaskStatus::Queued {
                t.status = TaskStatus::Paused;
                t.speed_bps = 0.0;
                t.eta_seconds = None;
            }
        }
        let _ = Storage::save_tasks(&tasks);
        drop(tasks);
        drop(controllers);

        self.schedule_next();
    }

    /// Resume all paused tasks.
    pub async fn resume_all(&self) {
        let task_ids: Vec<Uuid> = {
            let tasks = self.tasks.lock().await;
            tasks
                .iter()
                .filter(|t| t.status.can_resume())
                .map(|t| t.id)
                .collect()
        };

        for id in task_ids {
            self.start_task(id).await;
        }
    }

    /// Pause all running tasks.
    pub async fn pause_all(&self) {
        let task_ids: Vec<Uuid> = {
            let tasks = self.tasks.lock().await;
            tasks
                .iter()
                .filter(|t| t.status == TaskStatus::Downloading)
                .map(|t| t.id)
                .collect()
        };

        for id in task_ids {
            self.pause_task(id).await;
        }
    }

    /// Delete a task, optionally removing the downloaded file and metadata from disk.
    pub async fn delete_task(&self, task_id: Uuid, delete_file_on_disk: bool) {
        self.pause_task(task_id).await;

        let mut tasks = self.tasks.lock().await;
        if let Some(idx) = tasks.iter().position(|t| t.id == task_id) {
            let removed = tasks.remove(idx);
            let _ = Storage::save_tasks(&tasks);

            if delete_file_on_disk {
                let _ = std::fs::remove_file(&removed.save_path);
                DownloadMeta::remove_from_disk(&removed.save_path);
            }
        }
    }

    /// Redownload a task from scratch.
    pub async fn redownload_task(&self, task_id: Uuid) {
        self.pause_task(task_id).await;

        let mut tasks = self.tasks.lock().await;
        if let Some(t) = tasks.iter_mut().find(|t| t.id == task_id) {
            // Remove target file and meta
            let _ = std::fs::remove_file(&t.save_path);
            DownloadMeta::remove_from_disk(&t.save_path);

            t.downloaded_bytes = 0;
            t.status = TaskStatus::Queued;
            t.speed_bps = 0.0;
            t.eta_seconds = None;
            t.chunks.clear();
            t.completed_at = None;
            t.error_message = None;
        }
        let _ = Storage::save_tasks(&tasks);
        drop(tasks);

        self.start_task(task_id).await;
    }

    /// Schedule queued tasks if active downloads are below limit.
    pub fn schedule_next(&self) {
        let queue = self.clone();
        tokio::spawn(async move {
            let max_concurrent = queue.settings.lock().await.max_concurrent_downloads;
            let active_count = queue.active_controllers.lock().await.len();

            if active_count < max_concurrent {
                let tasks = queue.tasks.lock().await;
                let next_to_start = tasks
                    .iter()
                    .find(|t| t.status == TaskStatus::Queued)
                    .map(|t| t.id);
                drop(tasks);

                if let Some(id) = next_to_start {
                    queue.start_task(id).await;
                }
            }
        });
    }

    /// Update application settings and persist them.
    pub async fn update_settings(&self, new_settings: AppSettings) {
        let mut settings = self.settings.lock().await;
        *settings = new_settings;
        let _ = Storage::save_settings(&settings);
    }
}
