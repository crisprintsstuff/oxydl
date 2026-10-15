//! Media converter and audio extractor integration powered by FFmpeg.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertFormat {
    Mp3_320k,
    Mp3_192k,
    Aac,
    Flac,
    Wav,
    Mp4H264,
    MkvRemux,
    Webm,
    AudioCopy,
}

impl ConvertFormat {
    pub const ALL: &'static [ConvertFormat] = &[
        ConvertFormat::Mp3_320k,
        ConvertFormat::Mp3_192k,
        ConvertFormat::Aac,
        ConvertFormat::Flac,
        ConvertFormat::Wav,
        ConvertFormat::AudioCopy,
        ConvertFormat::Mp4H264,
        ConvertFormat::MkvRemux,
        ConvertFormat::Webm,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Mp3_320k => "MP3 Audio (320 kbps High Quality)",
            Self::Mp3_192k => "MP3 Audio (192 kbps Standard)",
            Self::Aac => "AAC Audio (.m4a)",
            Self::Flac => "FLAC Audio (Lossless)",
            Self::Wav => "WAV Audio (Uncompressed)",
            Self::AudioCopy => "Direct Audio Stream Copy (Fastest)",
            Self::Mp4H264 => "MP4 Video (H.264 + AAC Universal)",
            Self::MkvRemux => "MKV Video (Fast Remux)",
            Self::Webm => "WebM Video (VP9 + Opus)",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Mp3_320k | Self::Mp3_192k => "mp3",
            Self::Aac | Self::AudioCopy => "m4a",
            Self::Flac => "flac",
            Self::Wav => "wav",
            Self::Mp4H264 => "mp4",
            Self::MkvRemux => "mkv",
            Self::Webm => "webm",
        }
    }

    pub fn is_audio(&self) -> bool {
        match self {
            Self::Mp3_320k | Self::Mp3_192k | Self::Aac | Self::Flac | Self::Wav | Self::AudioCopy => true,
            Self::Mp4H264 | Self::MkvRemux | Self::Webm => false,
        }
    }

    pub fn build_args(&self, input: &Path, output: &Path) -> Vec<String> {
        let in_str = input.to_string_lossy().to_string();
        let out_str = output.to_string_lossy().to_string();

        let mut args = vec!["-y".to_string(), "-i".to_string(), in_str];

        match self {
            Self::Mp3_320k => {
                args.extend(["-vn".to_string(), "-c:a".to_string(), "libmp3lame".to_string(), "-b:a".to_string(), "320k".to_string()]);
            }
            Self::Mp3_192k => {
                args.extend(["-vn".to_string(), "-c:a".to_string(), "libmp3lame".to_string(), "-b:a".to_string(), "192k".to_string()]);
            }
            Self::Aac => {
                args.extend(["-vn".to_string(), "-c:a".to_string(), "aac".to_string(), "-b:a".to_string(), "192k".to_string()]);
            }
            Self::Flac => {
                args.extend(["-vn".to_string(), "-c:a".to_string(), "flac".to_string()]);
            }
            Self::Wav => {
                args.extend(["-vn".to_string(), "-c:a".to_string(), "pcm_s16le".to_string()]);
            }
            Self::AudioCopy => {
                args.extend(["-vn".to_string(), "-c:a".to_string(), "copy".to_string()]);
            }
            Self::Mp4H264 => {
                args.extend([
                    "-c:v".to_string(), "libx264".to_string(),
                    "-preset".to_string(), "fast".to_string(),
                    "-crf".to_string(), "22".to_string(),
                    "-c:a".to_string(), "aac".to_string(),
                    "-b:a".to_string(), "192k".to_string(),
                ]);
            }
            Self::MkvRemux => {
                args.extend(["-c".to_string(), "copy".to_string()]);
            }
            Self::Webm => {
                args.extend([
                    "-c:v".to_string(), "libvpx-vp9".to_string(),
                    "-crf".to_string(), "30".to_string(),
                    "-b:v".to_string(), "0".to_string(),
                    "-c:a".to_string(), "libopus".to_string(),
                ]);
            }
        }

        args.push(out_str);
        args
    }
}

#[derive(Debug, Clone)]
pub struct FfmpegInfo {
    pub binary_path: PathBuf,
    pub version: String,
}

pub struct FfmpegDetector;

impl FfmpegDetector {
    /// Discover FFmpeg binary from custom path, current working dir, or system PATH.
    pub fn find_ffmpeg(custom_path: Option<&Path>) -> Option<FfmpegInfo> {
        let binary_names = if cfg!(windows) {
            vec!["ffmpeg.exe", "ffmpeg"]
        } else {
            vec!["ffmpeg"]
        };

        let mut candidate_paths: Vec<PathBuf> = Vec::new();

        // 1. User specified custom path
        if let Some(cp) = custom_path {
            if cp.exists() {
                candidate_paths.push(cp.to_path_buf());
            }
        }

        // 2. Current application directory & bin subdirectories
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                for name in &binary_names {
                    candidate_paths.push(parent.join(name));
                    candidate_paths.push(parent.join("bin").join(name));
                    candidate_paths.push(parent.join("tools").join(name));
                }
            }
        }

        // 3. Current working directory
        if let Ok(cwd) = std::env::current_dir() {
            for name in &binary_names {
                candidate_paths.push(cwd.join(name));
                candidate_paths.push(cwd.join("bin").join(name));
            }
        }

        // 4. Windows AppData / ProgramFiles
        #[cfg(target_os = "windows")]
        {
            if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
                candidate_paths.push(PathBuf::from(local_app_data).join("OxyDL").join("bin").join("ffmpeg.exe"));
            }
            if let Ok(prog_files) = std::env::var("ProgramFiles") {
                candidate_paths.push(PathBuf::from(prog_files).join("ffmpeg").join("bin").join("ffmpeg.exe"));
            }
        }

        // 5. System PATH lookup
        if let Ok(path_var) = std::env::var("PATH") {
            let sep = if cfg!(windows) { ';' } else { ':' };
            for dir in path_var.split(sep) {
                for name in &binary_names {
                    candidate_paths.push(Path::new(dir).join(name));
                }
            }
        }

        // Test candidates
        for path in candidate_paths {
            if path.is_file() {
                if let Ok(output) = std::process::Command::new(&path).arg("-version").output() {
                    if output.status.success() {
                        let text = String::from_utf8_lossy(&output.stdout);
                        let first_line = text.lines().next().unwrap_or("ffmpeg version unknown");
                        let version = first_line.trim().to_string();
                        return Some(FfmpegInfo {
                            binary_path: path,
                            version,
                        });
                    }
                }
            }
        }

        None
    }
}

pub struct MediaConverter;

impl MediaConverter {
    /// Convert media file asynchronously with progress feedback
    pub async fn convert(
        ffmpeg_bin: &Path,
        input_path: &Path,
        output_path: &Path,
        format: ConvertFormat,
        cancel_token: Arc<AtomicBool>,
        progress_cb: impl Fn(f32, String) + Send + 'static,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !input_path.exists() {
            return Err("Input file does not exist".into());
        }

        if let Some(parent) = output_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }

        let args = format.build_args(input_path, output_path);

        let mut child = Command::new(ffmpeg_bin)
            .args(&args)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;

        let stderr = child.stderr.take().ok_or("Failed to open FFmpeg stderr pipe")?;
        let mut reader = BufReader::new(stderr).lines();

        let mut total_duration_secs: Option<f64> = None;

        while let Ok(Some(line)) = reader.next_line().await {
            if cancel_token.load(Ordering::Relaxed) {
                let _ = child.kill().await;
                let _ = tokio::fs::remove_file(output_path).await;
                return Err("Conversion cancelled by user".into());
            }

            // Parse Duration: 00:03:45.12
            if total_duration_secs.is_none() && line.contains("Duration:") {
                if let Some(pos) = line.find("Duration:") {
                    let rem = &line[pos + 9..];
                    if let Some(time_str) = rem.split(',').next() {
                        total_duration_secs = Self::parse_time_secs(time_str.trim());
                    }
                }
            }

            // Parse time=00:01:23.45 and speed=4.5x
            if line.contains("time=") {
                if let Some(t_idx) = line.find("time=") {
                    let rem = &line[t_idx + 5..];
                    let time_token = rem.split_whitespace().next().unwrap_or("");
                    if let Some(current_secs) = Self::parse_time_secs(time_token) {
                        let speed_str = if let Some(s_idx) = line.find("speed=") {
                            line[s_idx + 6..].split_whitespace().next().unwrap_or("1x").to_string()
                        } else {
                            "1x".to_string()
                        };

                        let frac = if let Some(tot) = total_duration_secs {
                            if tot > 0.0 {
                                (current_secs / tot).clamp(0.0, 1.0) as f32
                            } else {
                                0.5
                            }
                        } else {
                            0.0
                        };

                        let status = format!("{:.1}% (Speed: {})", frac * 100.0, speed_str);
                        progress_cb(frac, status);
                    }
                }
            }
        }

        let status = child.wait().await?;
        if !status.success() {
            return Err("FFmpeg process finished with an error".into());
        }

        progress_cb(1.0, "Complete".to_string());
        Ok(())
    }

    fn parse_time_secs(time_str: &str) -> Option<f64> {
        // Form: HH:MM:SS.ms
        let parts: Vec<&str> = time_str.split(':').collect();
        if parts.len() == 3 {
            let h = parts[0].parse::<f64>().ok()?;
            let m = parts[1].parse::<f64>().ok()?;
            let s = parts[2].parse::<f64>().ok()?;
            Some(h * 3600.0 + m * 60.0 + s)
        } else {
            None
        }
    }
}
