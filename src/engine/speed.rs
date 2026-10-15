//! Speed tracking and bandwidth throttling.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Debug)]
#[allow(dead_code)]
pub struct SpeedTracker {
    samples: VecDeque<(Instant, u64)>,
    window_duration: Duration,
    last_speed: f64,
    speed_limit_bps: Option<u64>,
}

impl SpeedTracker {
    pub fn new(window_secs: f32) -> Self {
        Self {
            samples: VecDeque::new(),
            window_duration: Duration::from_secs_f32(window_secs),
            last_speed: 0.0,
            speed_limit_bps: None,
        }
    }

    pub fn set_speed_limit(&mut self, limit_bps: Option<u64>) {
        self.speed_limit_bps = limit_bps;
    }

    /// Record newly downloaded bytes at this moment.
    pub fn record(&mut self, total_downloaded: u64) {
        let now = Instant::now();
        self.samples.push_back((now, total_downloaded));

        // Purge samples older than window_duration
        while let Some(&(time, _)) = self.samples.front() {
            if now.duration_since(time) > self.window_duration && self.samples.len() > 2 {
                self.samples.pop_front();
            } else {
                break;
            }
        }

        // Calculate speed from oldest sample in window to latest
        if self.samples.len() >= 2 {
            let (oldest_time, oldest_bytes) = *self.samples.front().unwrap();
            let (newest_time, newest_bytes) = *self.samples.back().unwrap();

            let elapsed = newest_time.duration_since(oldest_time).as_secs_f64();
            if elapsed > 0.05 && newest_bytes >= oldest_bytes {
                let bytes_diff = (newest_bytes - oldest_bytes) as f64;
                let instantaneous = bytes_diff / elapsed;
                // Exponential moving average smoothing
                self.last_speed = self.last_speed * 0.4 + instantaneous * 0.6;
            }
        }
    }

    /// Current smoothed speed in bytes per second.
    pub fn current_speed_bps(&self) -> f64 {
        self.last_speed
    }

    /// Calculate estimated seconds remaining.
    pub fn estimate_eta(&self, downloaded: u64, total: Option<u64>) -> Option<u64> {
        let total = total?;
        if downloaded >= total {
            return Some(0);
        }
        if self.last_speed <= 1.0 {
            return None;
        }
        let remaining_bytes = (total - downloaded) as f64;
        let secs = remaining_bytes / self.last_speed;
        Some(secs.ceil() as u64)
    }

    /// Throttler check: if rate limiting is enabled and we are exceeding it,
    /// compute how long the caller should sleep.
    pub fn throttle_delay(&self, bytes_transferred: usize, duration: Duration) -> Option<Duration> {
        if let Some(limit) = self.speed_limit_bps {
            if limit > 0 {
                let allowed_secs = (bytes_transferred as f64) / (limit as f64);
                let actual_secs = duration.as_secs_f64();
                if allowed_secs > actual_secs {
                    return Some(Duration::from_secs_f64(allowed_secs - actual_secs));
                }
            }
        }
        None
    }

    pub fn reset(&mut self) {
        self.samples.clear();
        self.last_speed = 0.0;
    }
}
