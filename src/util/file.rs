//! File utilities: category deduction, naming, pre-allocation, and app directories.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskCategory {
    All,
    Compressed,
    Documents,
    Music,
    Videos,
    Programs,
    Other,
}

impl TaskCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            TaskCategory::All => "All Downloads",
            TaskCategory::Compressed => "Compressed",
            TaskCategory::Documents => "Documents",
            TaskCategory::Music => "Music & Audio",
            TaskCategory::Videos => "Videos",
            TaskCategory::Programs => "Programs",
            TaskCategory::Other => "Other",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            TaskCategory::All => "📁",
            TaskCategory::Compressed => "📦",
            TaskCategory::Documents => "📄",
            TaskCategory::Music => "🎵",
            TaskCategory::Videos => "🎬",
            TaskCategory::Programs => "⚙",
            TaskCategory::Other => "📎",
        }
    }

    /// Deduce category from file extension.
    pub fn from_extension(ext: &str) -> Self {
        let ext_lower = ext.to_lowercase();
        match ext_lower.as_str() {
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "iso" | "tgz" | "z" | "lz" => {
                TaskCategory::Compressed
            }
            "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "epub" | "mobi"
            | "csv" | "rtf" | "odt" | "ods" => TaskCategory::Documents,
            "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" | "opus" | "aiff" => {
                TaskCategory::Music
            }
            "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" | "wmv" | "m4v" | "ts" | "3gp" => {
                TaskCategory::Videos
            }
            "exe" | "msi" | "apk" | "deb" | "rpm" | "dmg" | "pkg" | "appimage" | "bat" | "cmd" => {
                TaskCategory::Programs
            }
            _ => TaskCategory::Other,
        }
    }

    pub fn from_filename(filename: &str) -> Self {
        let path = Path::new(filename);
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            Self::from_extension(ext)
        } else {
            TaskCategory::Other
        }
    }
}

/// Extract suggested filename from URL or header value.
pub fn extract_filename_from_url(url_str: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url_str) {
        if let Some(segments) = parsed.path_segments() {
            let last = segments.last().unwrap_or("");
            if !last.trim().is_empty() {
                // Remove any url encoded parts
                if let Ok(decoded) = urlencoding::decode(last) {
                    let cleaned = sanitize_filename(decoded.trim());
                    if !cleaned.is_empty() {
                        return cleaned;
                    }
                }
            }
        }
    }
    "download.bin".to_string()
}

/// Parse Content-Disposition header if available.
pub fn parse_content_disposition(header_val: &str) -> Option<String> {
    // Looks like: attachment; filename="example.zip" or filename*=UTF-8''example.zip
    for part in header_val.split(';') {
        let part = part.trim();
        if let Some(stripped) = part.strip_prefix("filename*=") {
            let stripped = stripped.trim_matches('"');
            if let Some(pos) = stripped.find("''") {
                let encoded = &stripped[pos + 2..];
                if let Ok(decoded) = urlencoding::decode(encoded) {
                    let clean = sanitize_filename(&decoded);
                    if !clean.is_empty() {
                        return Some(clean);
                    }
                }
            }
        } else if let Some(stripped) = part.strip_prefix("filename=") {
            let clean = sanitize_filename(stripped.trim_matches('"').trim());
            if !clean.is_empty() {
                return Some(clean);
            }
        }
    }
    None
}

/// Remove dangerous or invalid characters for Windows/Linux filesystems.
pub fn sanitize_filename(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control() {
            out.push('_');
        } else {
            out.push(c);
        }
    }
    let trimmed = out.trim().to_string();
    if trimmed.is_empty() {
        "download.bin".to_string()
    } else {
        trimmed
    }
}

/// Detect application data directory.
/// In portable mode (portable.dat / portable folder next to exe), use `./data`.
/// Otherwise use system standard config/data dir (`%APPDATA%/oxydl` on Windows).
pub fn get_app_data_dir() -> PathBuf {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let portable_marker = exe_dir.join("portable.txt");
            let portable_data = exe_dir.join("data");
            if portable_marker.exists() || portable_data.exists() {
                let _ = std::fs::create_dir_all(&portable_data);
                return portable_data;
            }
        }
    }

    if let Some(proj_dirs) = directories::ProjectDirs::from("org", "OxyDL", "OxyDL") {
        let config_dir = proj_dirs.data_dir().to_path_buf();
        let _ = std::fs::create_dir_all(&config_dir);
        return config_dir;
    }

    PathBuf::from("./data")
}

/// Get default user downloads directory.
pub fn get_default_download_dir() -> PathBuf {
    if let Some(user_dirs) = directories::UserDirs::new() {
        if let Some(dl) = user_dirs.download_dir() {
            return dl.to_path_buf();
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Allocate a sparse file of the target size to allow instant random offset writing.
pub fn preallocate_file(path: &Path, size: u64) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    file.set_len(size)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_detection() {
        assert_eq!(TaskCategory::from_filename("archive.zip"), TaskCategory::Compressed);
        assert_eq!(TaskCategory::from_filename("song.mp3"), TaskCategory::Music);
        assert_eq!(TaskCategory::from_filename("movie.mkv"), TaskCategory::Videos);
        assert_eq!(TaskCategory::from_filename("setup.exe"), TaskCategory::Programs);
        assert_eq!(TaskCategory::from_filename("document.pdf"), TaskCategory::Documents);
    }

    #[test]
    fn test_extract_filename() {
        assert_eq!(
            extract_filename_from_url("https://example.com/files/test_setup_v1.0.exe?token=123"),
            "test_setup_v1.0.exe"
        );
    }

    #[test]
    fn test_content_disposition() {
        assert_eq!(
            parse_content_disposition("attachment; filename=\"installer.exe\""),
            Some("installer.exe".to_string())
        );
    }
}
