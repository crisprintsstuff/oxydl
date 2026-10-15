//! Pure asynchronous FTP download engine supporting passive mode and byte-level resume.

use std::net::SocketAddr;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom};
use tokio::net::TcpStream;
use url::Url;

use crate::engine::speed::SpeedLimiter;

pub struct FtpDownloadResult {
    pub total_bytes: Option<u64>,
    pub downloaded_bytes: u64,
}

pub struct FtpDownloader;

impl FtpDownloader {
    /// Download a file over FTP with byte-level resume and progress callbacks.
    pub async fn download(
        url_str: &str,
        save_path: &Path,
        initial_offset: u64,
        pause_signal: Arc<AtomicBool>,
        progress_cb: impl Fn(u64, Option<u64>, f64) + Send + 'static,
    ) -> Result<FtpDownloadResult, Box<dyn std::error::Error + Send + Sync>> {
        let parsed_url = Url::parse(url_str)?;
        let host = parsed_url.host_str().ok_or("FTP URL missing host")?;
        let port = parsed_url.port().unwrap_or(21);
        let path = parsed_url.path();
        let username = if parsed_url.username().is_empty() {
            "anonymous"
        } else {
            parsed_url.username()
        };
        let password = parsed_url.password().unwrap_or("anonymous@oxydl.org");

        // 1. Connect to FTP control port
        let mut ctrl_stream = TcpStream::connect(format!("{}:{}", host, port)).await?;
        let mut ctrl_buf = [0u8; 4096];

        // Read 220 banner
        let n = ctrl_stream.read(&mut ctrl_buf).await?;
        let banner = String::from_utf8_lossy(&ctrl_buf[..n]);
        if !banner.starts_with("220") {
            return Err(format!("FTP greeting failed: {}", banner).into());
        }

        // Send USER
        Self::send_cmd(&mut ctrl_stream, &format!("USER {}\r\n", username)).await?;
        let resp = Self::read_resp(&mut ctrl_stream, &mut ctrl_buf).await?;
        if resp.starts_with("331") {
            // Send PASS
            Self::send_cmd(&mut ctrl_stream, &format!("PASS {}\r\n", password)).await?;
            let pass_resp = Self::read_resp(&mut ctrl_stream, &mut ctrl_buf).await?;
            if !pass_resp.starts_with("230") {
                return Err(format!("FTP authentication failed: {}", pass_resp).into());
            }
        } else if !resp.starts_with("230") {
            return Err(format!("FTP USER rejected: {}", resp).into());
        }

        // Set binary mode (TYPE I)
        Self::send_cmd(&mut ctrl_stream, "TYPE I\r\n").await?;
        let _ = Self::read_resp(&mut ctrl_stream, &mut ctrl_buf).await?;

        // Query file size (SIZE <path>)
        Self::send_cmd(&mut ctrl_stream, &format!("SIZE {}\r\n", path)).await?;
        let size_resp = Self::read_resp(&mut ctrl_stream, &mut ctrl_buf).await?;
        let total_bytes: Option<u64> = if size_resp.starts_with("213") {
            size_resp.split_whitespace().nth(1).and_then(|s| s.parse::<u64>().ok())
        } else {
            None
        };

        // Enter passive mode (PASV)
        Self::send_cmd(&mut ctrl_stream, "PASV\r\n").await?;
        let pasv_resp = Self::read_resp(&mut ctrl_stream, &mut ctrl_buf).await?;
        let data_addr = Self::parse_pasv(&pasv_resp, host)?;

        // Connect to data socket
        let mut data_stream = TcpStream::connect(data_addr).await?;

        // Support resume (REST <offset>)
        let mut start_pos = initial_offset;
        if start_pos > 0 {
            Self::send_cmd(&mut ctrl_stream, &format!("REST {}\r\n", start_pos)).await?;
            let rest_resp = Self::read_resp(&mut ctrl_stream, &mut ctrl_buf).await?;
            if !rest_resp.starts_with("350") {
                // If server doesn't support REST, start from 0
                start_pos = 0;
            }
        }

        // Request file download (RETR <path>)
        Self::send_cmd(&mut ctrl_stream, &format!("RETR {}\r\n", path)).await?;
        let retr_resp = Self::read_resp(&mut ctrl_stream, &mut ctrl_buf).await?;
        if !retr_resp.starts_with("150") && !retr_resp.starts_with("125") {
            return Err(format!("FTP RETR rejected: {}", retr_resp).into());
        }

        // Open local destination file
        if let Some(parent) = save_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(save_path)
            .await?;

        file.seek(SeekFrom::Start(start_pos)).await?;

        // Stream data bytes
        let mut data_chunk = [0u8; 65536];
        let mut downloaded = start_pos;
        let mut speed_calc = SpeedLimiter::new();
        let mut last_report = std::time::Instant::now();

        loop {
            if pause_signal.load(Ordering::Relaxed) {
                let _ = Self::send_cmd(&mut ctrl_stream, "ABOR\r\n").await;
                break;
            }

            let bytes_read = data_stream.read(&mut data_chunk).await?;
            if bytes_read == 0 {
                break; // End of transfer
            }

            file.write_all(&data_chunk[..bytes_read]).await?;
            downloaded += bytes_read as u64;
            speed_calc.record(bytes_read);

            if last_report.elapsed() >= std::time::Duration::from_millis(200) {
                let speed = speed_calc.current_speed_bps();
                progress_cb(downloaded, total_bytes, speed);
                last_report = std::time::Instant::now();
            }
        }

        file.flush().await?;
        let _ = Self::send_cmd(&mut ctrl_stream, "QUIT\r\n").await;

        Ok(FtpDownloadResult {
            total_bytes,
            downloaded_bytes: downloaded,
        })
    }

    async fn send_cmd(stream: &mut TcpStream, cmd: &str) -> Result<(), std::io::Error> {
        stream.write_all(cmd.as_bytes()).await
    }

    async fn read_resp(stream: &mut TcpStream, buf: &mut [u8]) -> Result<String, std::io::Error> {
        let n = stream.read(buf).await?;
        Ok(String::from_utf8_lossy(&buf[..n]).to_string())
    }

    fn parse_pasv(resp: &str, fallback_host: &str) -> Result<SocketAddr, Box<dyn std::error::Error + Send + Sync>> {
        let open_paren = resp.find('(').ok_or("Malformed PASV response")?;
        let close_paren = resp.find(')').ok_or("Malformed PASV response")?;
        let inside = &resp[open_paren + 1..close_paren];
        let nums: Vec<u16> = inside
            .split(',')
            .filter_map(|s| s.trim().parse::<u16>().ok())
            .collect();

        if nums.len() != 6 {
            return Err("Invalid PASV parameters".into());
        }

        let ip_str = format!("{}.{}.{}.{}", nums[0], nums[1], nums[2], nums[3]);
        let port = nums[4] * 256 + nums[5];

        let addr_str = format!("{}:{}", ip_str, port);
        if let Ok(addr) = addr_str.parse::<SocketAddr>() {
            Ok(addr)
        } else {
            let fallback_addr = format!("{}:{}", fallback_host, port);
            Ok(fallback_addr.parse()?)
        }
    }
}
