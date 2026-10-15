//! Local HTTP API server enabling browser extensions (Chrome, Firefox, Edge) to send downloads to Oxidm.

use std::net::SocketAddr;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::manager::queue::QueueManager;

pub const DEFAULT_API_PORT: u16 = 19532; // O-X-I-D-M port

#[derive(Debug, Deserialize)]
pub struct AddDownloadRequest {
    pub url: String,
    pub file_name: Option<String>,
    pub num_connections: Option<usize>,
    pub start_now: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: Option<String>,
    pub data: Option<T>,
}

pub struct ApiServer {
    queue: QueueManager,
    port: u16,
}

impl ApiServer {
    pub fn new(queue: QueueManager, port: u16) -> Self {
        Self { queue, port }
    }

    /// Start the background HTTP listener on 127.0.0.1:19532
    pub fn start(self: Arc<Self>) {
        tokio::spawn(async move {
            let addr = SocketAddr::from(([127, 0, 0, 1], self.port));
            let listener = match TcpListener::bind(addr).await {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("[Oxidm API] Could not bind to {}: {}", addr, e);
                    return;
                }
            };

            loop {
                if let Ok((stream, _)) = listener.accept().await {
                    let server = self.clone();
                    tokio::spawn(async move {
                        let _ = server.handle_connection(stream).await;
                    });
                }
            }
        });
    }

    async fn handle_connection(&self, mut stream: TcpStream) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut buffer = [0u8; 8192];
        let bytes_read = stream.read(&mut buffer).await?;
        if bytes_read == 0 {
            return Ok(());
        }

        let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);
        let mut lines = request_str.lines();
        let request_line = lines.next().unwrap_or("");
        let mut parts = request_line.split_whitespace();
        let method = parts.next().unwrap_or("");
        let path = parts.next().unwrap_or("");

        // Handle CORS Preflight
        if method == "OPTIONS" {
            let response = "HTTP/1.1 204 No Content\r\n\
                Access-Control-Allow-Origin: *\r\n\
                Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
                Access-Control-Allow-Headers: Content-Type, Authorization\r\n\
                Access-Control-Max-Age: 86400\r\n\
                Connection: close\r\n\r\n";
            stream.write_all(response.as_bytes()).await?;
            return Ok(());
        }

        // GET /status (Extension connection check)
        if method == "GET" && (path == "/status" || path == "/") {
            let tasks = self.queue.tasks.lock().await;
            let active_count = tasks.iter().filter(|t| t.status == crate::manager::task::TaskStatus::Downloading).count();
            let total_count = tasks.len();
            drop(tasks);

            let body = serde_json::json!({
                "running": true,
                "app": "Oxidm",
                "version": env!("CARGO_PKG_VERSION"),
                "active_downloads": active_count,
                "total_downloads": total_count,
            });

            self.send_json_response(&mut stream, 200, &body).await?;
            return Ok(());
        }

        // POST /add (Send download from browser extension)
        if method == "POST" && path == "/add" {
            // Find body start (after double newline)
            if let Some(pos) = request_str.find("\r\n\r\n") {
                let body_slice = &request_str[pos + 4..];
                if let Ok(req) = serde_json::from_str::<AddDownloadRequest>(body_slice.trim()) {
                    let url = req.url.trim().to_string();
                    if !url.is_empty() {
                        let file_name = req.file_name.unwrap_or_else(|| {
                            crate::util::file::extract_filename_from_url(&url)
                        });
                        let save_dir = {
                            let s = self.queue.settings.lock().await;
                            s.default_save_dir.clone()
                        };
                        let connections = req.num_connections.unwrap_or_else(|| {
                            8
                        });
                        let start_now = req.start_now.unwrap_or(true);

                        let task_id = self.queue.add_task(url, file_name, save_dir, connections, start_now).await;

                        let resp = serde_json::json!({
                            "success": true,
                            "task_id": task_id.to_string(),
                            "message": "Download added to Oxidm successfully",
                        });
                        self.send_json_response(&mut stream, 200, &resp).await?;
                        return Ok(());
                    }
                }
            }

            let err = serde_json::json!({
                "success": false,
                "error": "Invalid request payload. 'url' is required.",
            });
            self.send_json_response(&mut stream, 400, &err).await?;
            return Ok(());
        }

        // 404 Not Found
        let not_found = serde_json::json!({
            "success": false,
            "error": "Endpoint not found",
        });
        self.send_json_response(&mut stream, 404, &not_found).await?;
        Ok(())
    }

    async fn send_json_response(
        &self,
        stream: &mut TcpStream,
        status_code: u16,
        data: &serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let json_body = serde_json::to_string(data)?;
        let status_text = match status_code {
            200 => "OK",
            400 => "Bad Request",
            404 => "Not Found",
            _ => "Status",
        };

        let response = format!(
            "HTTP/1.1 {} {}\r\n\
            Content-Type: application/json; charset=utf-8\r\n\
            Content-Length: {}\r\n\
            Access-Control-Allow-Origin: *\r\n\
            Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
            Access-Control-Allow-Headers: Content-Type\r\n\
            Connection: close\r\n\r\n{}",
            status_code,
            status_text,
            json_body.len(),
            json_body
        );

        stream.write_all(response.as_bytes()).await?;
        Ok(())
    }
}
