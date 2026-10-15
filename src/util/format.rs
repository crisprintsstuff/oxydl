//! Formatting utilities for bytes, speed, ETA, and percentages.

/// Format raw byte count into a human-readable string (e.g., 1.45 GB, 320 KB).
pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;

    let b = bytes as f64;
    if b >= TB {
        format!("{:.2} TB", b / TB)
    } else if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.2} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

/// Format transfer speed in bytes per second into human-readable string (e.g., 5.4 MB/s).
pub fn format_speed(bps: f64) -> String {
    if bps <= 0.0 {
        return "0 KB/s".to_string();
    }
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    if bps >= GB {
        format!("{:.2} GB/s", bps / GB)
    } else if bps >= MB {
        format!("{:.2} MB/s", bps / MB)
    } else if bps >= KB {
        format!("{:.1} KB/s", bps / KB)
    } else {
        format!("{:.0} B/s", bps)
    }
}

/// Format estimated remaining time (e.g., "1m 32s", "4h 12m", "--").
pub fn format_eta(seconds: Option<u64>) -> String {
    match seconds {
        None => "--".to_string(),
        Some(0) => "Done".to_string(),
        Some(s) if s < 60 => format!("{}s", s),
        Some(s) if s < 3600 => {
            let m = s / 60;
            let rem_s = s % 60;
            format!("{}m {}s", m, rem_s)
        }
        Some(s) => {
            let h = s / 3600;
            let m = (s % 3600) / 60;
            format!("{}h {}m", h, m)
        }
    }
}

/// Calculate 0.0 .. 1.0 fraction for progress bars.
#[allow(dead_code)]
pub fn calculate_fraction(downloaded: u64, total: Option<u64>) -> f32 {
    match total {
        Some(tot) if tot > 0 => ((downloaded as f64) / (tot as f64)).clamp(0.0, 1.0) as f32,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }

    #[test]
    fn test_format_speed() {
        assert_eq!(format_speed(0.0), "0 KB/s");
        assert_eq!(format_speed(2048.0), "2.0 KB/s");
        assert_eq!(format_speed(1048576.0 * 2.5), "2.50 MB/s");
    }

    #[test]
    fn test_format_eta() {
        assert_eq!(format_eta(None), "--");
        assert_eq!(format_eta(Some(0)), "Done");
        assert_eq!(format_eta(Some(45)), "45s");
        assert_eq!(format_eta(Some(125)), "2m 5s");
        assert_eq!(format_eta(Some(3665)), "1h 1m");
    }
}
