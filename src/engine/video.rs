//! Direct video streaming parser and format extractor for YouTube and Vimeo.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoProvider {
    YouTube,
    Vimeo,
}

impl VideoProvider {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::YouTube => "YouTube",
            Self::Vimeo => "Vimeo",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::YouTube => "▶",
            Self::Vimeo => "🎬",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoStream {
    pub quality_label: String, // e.g., "1080p (Full HD)", "720p (HD)", "Audio (128 kbps)"
    pub direct_url: String,
    pub extension: String,      // "mp4", "webm", "m4a"
    pub file_size: Option<u64>,
    pub is_audio_only: bool,
    pub has_audio: bool,
    pub has_video: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoInfo {
    pub provider: VideoProvider,
    pub video_id: String,
    pub title: String,
    pub author: String,
    pub duration_secs: Option<u64>,
    pub thumbnail_url: Option<String>,
    pub streams: Vec<VideoStream>,
}

pub struct VideoExtractor;

impl VideoExtractor {
    /// Check if a given URL is a supported video stream platform
    pub fn is_supported(url: &str) -> bool {
        let lower = url.trim().to_lowercase();
        lower.contains("youtube.com/watch")
            || lower.contains("youtu.be/")
            || lower.contains("youtube.com/shorts/")
            || lower.contains("youtube.com/embed/")
            || lower.contains("vimeo.com/")
    }

    /// Extract video metadata and direct CDN streaming URLs
    pub async fn extract(url_str: &str) -> Result<VideoInfo, Box<dyn std::error::Error + Send + Sync>> {
        let trimmed = url_str.trim();
        let lower = trimmed.to_lowercase();

        if lower.contains("vimeo.com") {
            Self::extract_vimeo(trimmed).await
        } else if lower.contains("youtube.com") || lower.contains("youtu.be") {
            Self::extract_youtube(trimmed).await
        } else {
            Err("Unsupported streaming video provider".into())
        }
    }

    /// Extract direct progressive MP4 streams from Vimeo
    async fn extract_vimeo(url_str: &str) -> Result<VideoInfo, Box<dyn std::error::Error + Send + Sync>> {
        let video_id = Self::extract_vimeo_id(url_str)
            .ok_or("Could not parse Vimeo video ID from URL")?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(12))
            .build()?;

        let config_url = format!("https://player.vimeo.com/video/{}/config", video_id);
        let resp = client.get(&config_url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) OxyDL/0.1.0")
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(format!("Vimeo returned HTTP {}", resp.status()).into());
        }

        let json_val: Value = resp.json().await?;

        let title = json_val["video"]["title"].as_str().unwrap_or("Vimeo Video").to_string();
        let author = json_val["video"]["owner"]["name"].as_str().unwrap_or("Vimeo Creator").to_string();
        let duration_secs = json_val["video"]["duration"].as_u64();
        let thumbnail_url = json_val["video"]["thumbs"]["base"]
            .as_str()
            .or_else(|| json_val["video"]["thumbs"]["640"].as_str())
            .map(|s| s.to_string());

        let mut streams = Vec::new();

        // 1. Progressive MP4 streams (Direct CDN files)
        if let Some(progressive) = json_val["request"]["files"]["progressive"].as_array() {
            for item in progressive {
                if let Some(direct_url) = item["url"].as_str() {
                    let quality = item["quality"].as_str().unwrap_or("720p");
                    let width = item["width"].as_u64().unwrap_or(0);
                    let height = item["height"].as_u64().unwrap_or(0);
                    let fps = item["fps"].as_u64().unwrap_or(30);

                    let label = if width > 0 && height > 0 {
                        format!("{} ({}x{}, {}fps)", quality, width, height, fps)
                    } else {
                        format!("{} MP4", quality)
                    };

                    streams.push(VideoStream {
                        quality_label: label,
                        direct_url: direct_url.to_string(),
                        extension: "mp4".to_string(),
                        file_size: None,
                        is_audio_only: false,
                        has_audio: true,
                        has_video: true,
                    });
                }
            }
        }

        // 2. HLS stream fallback (.m3u8 master playlist)
        if let Some(hls_url) = json_val["request"]["files"]["hls"]["cdns"]["fastly_skyfire"]["url"]
            .as_str()
            .or_else(|| json_val["request"]["files"]["hls"]["default_cdn"].as_str())
        {
            streams.push(VideoStream {
                quality_label: "Adaptive HLS Stream (Master .m3u8)".to_string(),
                direct_url: hls_url.to_string(),
                extension: "mp4".to_string(),
                file_size: None,
                is_audio_only: false,
                has_audio: true,
                has_video: true,
            });
        }

        if streams.is_empty() {
            return Err("No downloadable video streams found for this Vimeo video".into());
        }

        Ok(VideoInfo {
            provider: VideoProvider::Vimeo,
            video_id,
            title,
            author,
            duration_secs,
            thumbnail_url,
            streams,
        })
    }

    /// Extract direct CDN streams from YouTube using InnerTube API
    async fn extract_youtube(url_str: &str) -> Result<VideoInfo, Box<dyn std::error::Error + Send + Sync>> {
        let video_id = Self::extract_youtube_id(url_str)
            .ok_or("Could not parse YouTube video ID from URL")?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(12))
            .build()?;

        // Query YouTube InnerTube Android client (returns direct URLs without signature cipher)
        let player_api_url = "https://www.youtube.com/youtubei/v1/player";
        let payload = serde_json::json!({
            "videoId": video_id,
            "context": {
                "client": {
                    "clientName": "ANDROID",
                    "clientVersion": "19.09.37",
                    "androidSdkVersion": 30,
                    "hl": "en",
                    "gl": "US"
                }
            }
        });

        let resp = client.post(player_api_url)
            .header("Content-Type", "application/json")
            .header("User-Agent", "com.google.android.youtube/19.09.37 (Linux; U; Android 11) gzip")
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(format!("YouTube API returned HTTP {}", resp.status()).into());
        }

        let json_val: Value = resp.json().await?;

        // Check if video is playable
        let playability = json_val["playabilityStatus"]["status"].as_str().unwrap_or("OK");
        if playability != "OK" {
            let reason = json_val["playabilityStatus"]["reason"].as_str().unwrap_or("Video is unavailable or private");
            return Err(format!("YouTube error: {}", reason).into());
        }

        let details = &json_val["videoDetails"];
        let title = details["title"].as_str().unwrap_or("YouTube Video").to_string();
        let author = details["author"].as_str().unwrap_or("YouTube Channel").to_string();
        let duration_secs = details["lengthSeconds"].as_str().and_then(|s| s.parse::<u64>().ok());

        let thumbnail_url = details["thumbnail"]["thumbnails"]
            .as_array()
            .and_then(|arr| arr.last())
            .and_then(|t| t["url"].as_str())
            .map(|s| s.to_string());

        let mut streams = Vec::new();

        // 1. Muxed formats (Video + Audio combined, usually 720p and 360p)
        if let Some(formats) = json_val["streamingData"]["formats"].as_array() {
            for f in formats {
                if let Some(url) = f["url"].as_str() {
                    let quality = f["qualityLabel"].as_str().unwrap_or("Standard");
                    let mime = f["mimeType"].as_str().unwrap_or("video/mp4");
                    let ext = if mime.contains("webm") { "webm" } else { "mp4" };
                    let size = f["contentLength"].as_str().and_then(|s| s.parse::<u64>().ok());

                    streams.push(VideoStream {
                        quality_label: format!("{} (Muxed Video+Audio)", quality),
                        direct_url: url.to_string(),
                        extension: ext.to_string(),
                        file_size: size,
                        is_audio_only: false,
                        has_audio: true,
                        has_video: true,
                    });
                }
            }
        }

        // 2. Adaptive formats (HD/4K video streams and audio-only streams)
        if let Some(adaptives) = json_val["streamingData"]["adaptiveFormats"].as_array() {
            for f in adaptives {
                if let Some(url) = f["url"].as_str() {
                    let mime = f["mimeType"].as_str().unwrap_or("");
                    let size = f["contentLength"].as_str().and_then(|s| s.parse::<u64>().ok());

                    if mime.starts_with("audio/") {
                        let bitrate = f["bitrate"].as_u64().unwrap_or(128000) / 1000;
                        let ext = if mime.contains("mp4") { "m4a" } else { "webm" };
                        streams.push(VideoStream {
                            quality_label: format!("🎵 Audio Only (~{} kbps)", bitrate),
                            direct_url: url.to_string(),
                            extension: ext.to_string(),
                            file_size: size,
                            is_audio_only: true,
                            has_audio: true,
                            has_video: false,
                        });
                    } else if let Some(quality) = f["qualityLabel"].as_str() {
                        let fps = f["fps"].as_u64().unwrap_or(30);
                        let ext = if mime.contains("webm") { "webm" } else { "mp4" };
                        streams.push(VideoStream {
                            quality_label: format!("🎬 {} ({}fps)", quality, fps),
                            direct_url: url.to_string(),
                            extension: ext.to_string(),
                            file_size: size,
                            is_audio_only: false,
                            has_audio: false,
                            has_video: true,
                        });
                    }
                }
            }
        }

        if streams.is_empty() {
            return Err("No direct stream URLs could be extracted for this YouTube video".into());
        }

        Ok(VideoInfo {
            provider: VideoProvider::YouTube,
            video_id,
            title,
            author,
            duration_secs,
            thumbnail_url,
            streams,
        })
    }

    /// Parse YouTube 11-char video ID from various URL patterns
    pub fn extract_youtube_id(url: &str) -> Option<String> {
        let trimmed = url.trim();

        // 1. https://youtu.be/ID
        if let Some(idx) = trimmed.find("youtu.be/") {
            let rest = &trimmed[idx + 9..];
            let id: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-').collect();
            if id.len() >= 10 {
                return Some(id);
            }
        }

        // 2. https://www.youtube.com/watch?v=ID
        if let Some(idx) = trimmed.find("v=") {
            let rest = &trimmed[idx + 2..];
            let id: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-').collect();
            if id.len() >= 10 {
                return Some(id);
            }
        }

        // 3. https://www.youtube.com/shorts/ID or /embed/ID
        for prefix in &["/shorts/", "/embed/", "/v/"] {
            if let Some(idx) = trimmed.find(prefix) {
                let rest = &trimmed[idx + prefix.len()..];
                let id: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-').collect();
                if id.len() >= 10 {
                    return Some(id);
                }
            }
        }

        None
    }

    /// Parse Vimeo numeric video ID
    pub fn extract_vimeo_id(url: &str) -> Option<String> {
        let trimmed = url.trim();
        let parts: Vec<&str> = trimmed.split('?').next().unwrap_or("").split('/').collect();
        for part in parts.iter().rev() {
            if !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()) {
                return Some(part.to_string());
            }
        }
        None
    }
}
