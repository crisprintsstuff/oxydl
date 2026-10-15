//! Scheduled download queues, start/stop timers, and peak-hours speed limiting.

use std::sync::Arc;
use std::time::Duration;
use chrono::{Datelike, Local, NaiveTime, Timelike};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::manager::queue::QueueManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScheduleAction {
    None,
    PauseQueue,
    SleepComputer,
    ShutdownComputer,
}

impl ScheduleAction {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::None => "Do Nothing",
            Self::PauseQueue => "Pause Download Queue",
            Self::SleepComputer => "Put Computer to Sleep",
            Self::ShutdownComputer => "Shut Down Computer",
        }
    }
}

fn default_false() -> bool { false }
fn default_start_time() -> String { "02:00".to_string() }
fn default_stop_time() -> String { "07:00".to_string() }
fn default_all_days() -> Vec<u8> { vec![0, 1, 2, 3, 4, 5, 6] }
fn default_weekdays() -> Vec<u8> { vec![0, 1, 2, 3, 4] }
fn default_peak_start() -> String { "09:00".to_string() }
fn default_peak_end() -> String { "18:00".to_string() }
fn default_peak_limit() -> u64 { 500 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueSchedule {
    #[serde(default = "default_false")]
    pub enabled: bool,
    #[serde(default = "default_start_time")]
    pub start_time: String, // "HH:MM"
    #[serde(default = "default_stop_time")]
    pub stop_time: String,  // "HH:MM"
    #[serde(default = "default_all_days")]
    pub days_of_week: Vec<u8>, // 0=Mon, 1=Tue, ..., 6=Sun
    #[serde(default)]
    pub one_shot_date: Option<String>, // "YYYY-MM-DD"
    #[serde(default = "default_action_none")]
    pub action_on_complete: ScheduleAction,
}

fn default_action_none() -> ScheduleAction {
    ScheduleAction::None
}

impl Default for QueueSchedule {
    fn default() -> Self {
        Self {
            enabled: false,
            start_time: default_start_time(),
            stop_time: default_stop_time(),
            days_of_week: default_all_days(),
            one_shot_date: None,
            action_on_complete: ScheduleAction::None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeedSchedule {
    #[serde(default = "default_false")]
    pub enabled: bool,
    #[serde(default = "default_peak_start")]
    pub peak_start: String, // "09:00"
    #[serde(default = "default_peak_end")]
    pub peak_end: String,   // "18:00"
    #[serde(default = "default_peak_limit")]
    pub peak_limit_kbps: u64, // e.g. 500 KB/s
    #[serde(default)]
    pub off_peak_limit_kbps: Option<u64>, // None = unlimited
    #[serde(default = "default_weekdays")]
    pub peak_days: Vec<u8>, // 0=Mon ... 4=Fri
}

impl Default for SpeedSchedule {
    fn default() -> Self {
        Self {
            enabled: false,
            peak_start: default_peak_start(),
            peak_end: default_peak_end(),
            peak_limit_kbps: default_peak_limit(),
            off_peak_limit_kbps: None,
            peak_days: default_weekdays(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchedulerConfig {
    #[serde(default)]
    pub queue_schedule: QueueSchedule,
    #[serde(default)]
    pub speed_schedule: SpeedSchedule,
}

pub struct SchedulerService {
    queue: QueueManager,
    config: Arc<Mutex<SchedulerConfig>>,
    last_action_minute: Arc<Mutex<Option<(u32, u32)>>>, // (hour, minute) to prevent re-firing in same minute
}

impl SchedulerService {
    pub fn new(queue: QueueManager, initial_config: SchedulerConfig) -> Self {
        Self {
            queue,
            config: Arc::new(Mutex::new(initial_config)),
            last_action_minute: Arc::new(Mutex::new(None)),
        }
    }

    pub fn get_config(&self) -> Arc<Mutex<SchedulerConfig>> {
        self.config.clone()
    }

    /// Spawn the background scheduler evaluation loop.
    pub fn start(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(2));
            loop {
                interval.tick().await;
                self.evaluate_schedules().await;
            }
        });
    }

    pub async fn update_config(&self, new_config: SchedulerConfig) {
        let mut cfg = self.config.lock().await;
        *cfg = new_config;
    }

    async fn evaluate_schedules(&self) {
        let now = Local::now();
        let current_weekday = now.weekday().num_days_from_monday() as u8; // 0=Mon..6=Sun
        let current_hour = now.hour();
        let current_min = now.minute();
        let current_time = NaiveTime::from_hms_opt(current_hour, current_min, now.second()).unwrap();

        let cfg = self.config.lock().await.clone();

        // 1. Evaluate Peak-Hours Speed Limiter
        if cfg.speed_schedule.enabled {
            if let (Ok(p_start), Ok(p_end)) = (
                NaiveTime::parse_from_str(&cfg.speed_schedule.peak_start, "%H:%M"),
                NaiveTime::parse_from_str(&cfg.speed_schedule.peak_end, "%H:%M"),
            ) {
                let is_peak_day = cfg.speed_schedule.peak_days.contains(&current_weekday);
                let is_peak_time = if p_start <= p_end {
                    current_time >= p_start && current_time < p_end
                } else {
                    // Spans midnight, e.g. 22:00 to 06:00
                    current_time >= p_start || current_time < p_end
                };

                let target_limit = if is_peak_day && is_peak_time {
                    Some(cfg.speed_schedule.peak_limit_kbps)
                } else {
                    cfg.speed_schedule.off_peak_limit_kbps
                };

                // Check and update queue speed limit if different
                let mut settings = self.queue.settings.lock().await;
                if settings.speed_limit_kbps != target_limit {
                    settings.speed_limit_kbps = target_limit;
                    let clone = settings.clone();
                    drop(settings);
                    self.queue.update_settings(clone).await;
                }
            }
        }

        // 2. Evaluate Queue Start/Stop Timers
        if cfg.queue_schedule.enabled {
            let is_active_day = cfg.queue_schedule.days_of_week.contains(&current_weekday);

            // Check optional one-shot date
            let is_active_date = if let Some(ref d_str) = cfg.queue_schedule.one_shot_date {
                now.format("%Y-%m-%d").to_string() == *d_str
            } else {
                true
            };

            if is_active_day && is_active_date {
                let mut last_action = self.last_action_minute.lock().await;
                let is_new_minute = *last_action != Some((current_hour, current_min));

                if is_new_minute {
                    // Check Start Time
                    if let Ok(start_t) = NaiveTime::parse_from_str(&cfg.queue_schedule.start_time, "%H:%M") {
                        if current_hour == start_t.hour() && current_min == start_t.minute() {
                            *last_action = Some((current_hour, current_min));
                            self.queue.resume_all().await;
                        }
                    }

                    // Check Stop Time
                    if let Ok(stop_t) = NaiveTime::parse_from_str(&cfg.queue_schedule.stop_time, "%H:%M") {
                        if current_hour == stop_t.hour() && current_min == stop_t.minute() {
                            *last_action = Some((current_hour, current_min));
                            self.queue.pause_all().await;
                        }
                    }
                }
            }

            // Check post-completion action
            if cfg.queue_schedule.action_on_complete != ScheduleAction::None {
                let tasks = self.queue.tasks.lock().await;
                let has_downloads = !tasks.is_empty();
                let all_finished = has_downloads && tasks.iter().all(|t| {
                    t.status == crate::manager::task::TaskStatus::Completed
                });

                if all_finished {
                    let action = cfg.queue_schedule.action_on_complete;
                    drop(tasks);
                    Self::execute_system_action(action).await;
                }
            }
        }
    }

    /// Execute system actions like Sleep or Shutdown
    pub async fn execute_system_action(action: ScheduleAction) {
        match action {
            ScheduleAction::None => {}
            ScheduleAction::PauseQueue => {}
            ScheduleAction::SleepComputer => {
                #[cfg(target_os = "windows")]
                {
                    let _ = tokio::process::Command::new("rundll32.exe")
                        .args(["powrprof.dll,SetSuspendState", "0,1,0"])
                        .spawn();
                }
                #[cfg(not(target_os = "windows"))]
                {
                    let _ = tokio::process::Command::new("systemctl")
                        .arg("suspend")
                        .spawn();
                }
            }
            ScheduleAction::ShutdownComputer => {
                #[cfg(target_os = "windows")]
                {
                    let _ = tokio::process::Command::new("shutdown")
                        .args(["/s", "/t", "30", "/c", "OxyDL: Downloads completed, shutting down."])
                        .spawn();
                }
                #[cfg(not(target_os = "windows"))]
                {
                    let _ = tokio::process::Command::new("shutdown")
                        .args(["-h", "+1", "OxyDL: Downloads completed, shutting down."])
                        .spawn();
                }
            }
        }
    }
}
