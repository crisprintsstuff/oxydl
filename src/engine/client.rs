//! HTTP client configuration, URL probing, and header inspection.

use std::time::Duration;
use reqwest::header::{ACCEPT_RANGES, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_RANGE, RANGE};
use crate::util::file::{extract_filename_from_url, parse_content_disposition};

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36 Oxidm/0.1.0";

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ProbeResult {
    pub url: String,
    pub suggested_filename: String,
    pub total_bytes: Option<u64>,
    pub supports_ranges: bool,
    pub content_type: Option<String>,
}

/// Create a standardized Reqwest client with rustls-tls, redirect following, and timeouts.
pub fn create_http_client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .redirect(reqwest::redirect::Policy::limited(10))
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(120))
        .build()
}

/// Probe a remote URL to discover its file size, range support, and suggested filename.
pub async fn probe_url(client: &reqwest::Client, url: &str) -> Result<ProbeResult, String> {
    // 1. Try HEAD request first
    let head_res = client.head(url).send().await;

    if let Ok(resp) = head_res {
        if resp.status().is_success() {
            let headers = resp.headers();

            let total_bytes = headers
                .get(CONTENT_LENGTH)
                .and_then(|val| val.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());

            let supports_ranges = headers
                .get(ACCEPT_RANGES)
                .and_then(|val| val.to_str().ok())
                .map(|val| val.to_ascii_lowercase().contains("bytes"))
                .unwrap_or(false);

            let content_type = headers
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|val| val.to_str().ok())
                .map(|s| s.to_string());

            let mut filename = None;
            if let Some(cd) = headers.get(CONTENT_DISPOSITION).and_then(|v| v.to_str().ok()) {
                filename = parse_content_disposition(cd);
            }
            let suggested_filename = filename.unwrap_or_else(|| extract_filename_from_url(url));

            if total_bytes.is_some() {
                return Ok(ProbeResult {
                    url: url.to_string(),
                    suggested_filename,
                    total_bytes,
                    supports_ranges,
                    content_type,
                });
            }
        }
    }

    // 2. If HEAD failed or gave no content-length, send GET with Range: bytes=0-0
    let get_range_res = client
        .get(url)
        .header(RANGE, "bytes=0-0")
        .send()
        .await
        .map_err(|e| format!("Network request failed: {}", e))?;

    let headers = get_range_res.headers();
    let status = get_range_res.status();

    let supports_ranges = status == reqwest::StatusCode::PARTIAL_CONTENT
        || headers
            .get(ACCEPT_RANGES)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_ascii_lowercase().contains("bytes"))
            .unwrap_or(false);

    let mut total_bytes = None;
    if let Some(cr) = headers.get(CONTENT_RANGE).and_then(|v| v.to_str().ok()) {
        // e.g. bytes 0-0/10485760
        if let Some(slash_idx) = cr.rfind('/') {
            let total_str = &cr[slash_idx + 1..];
            total_bytes = total_str.parse::<u64>().ok();
        }
    }

    if total_bytes.is_none() {
        total_bytes = headers
            .get(CONTENT_LENGTH)
            .and_then(|val| val.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());
    }

    let content_type = headers
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|val| val.to_str().ok())
        .map(|s| s.to_string());

    let mut filename = None;
    if let Some(cd) = headers.get(CONTENT_DISPOSITION).and_then(|v| v.to_str().ok()) {
        filename = parse_content_disposition(cd);
    }
    let suggested_filename = filename.unwrap_or_else(|| extract_filename_from_url(url));

    Ok(ProbeResult {
        url: url.to_string(),
        suggested_filename,
        total_bytes,
        supports_ranges,
        content_type,
    })
}
