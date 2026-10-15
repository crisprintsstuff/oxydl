//! Model Context Protocol (MCP) Server for OxyDL.
//! Enables AI assistants (Claude, Cursor, Antigravity) to query and control OxyDL downloads.

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use uuid::Uuid;

use crate::manager::queue::QueueManager;
use crate::manager::task::TaskStatus;
use crate::util::format::{format_bytes, format_eta, format_speed};

#[derive(Debug, Deserialize, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
}

pub struct McpServer {
    queue: QueueManager,
}

impl McpServer {
    pub fn new(queue: QueueManager) -> Self {
        Self { queue }
    }

    /// Process a single JSON-RPC message and return response if needed.
    pub async fn handle_request(&self, req: JsonRpcRequest) -> Option<JsonRpcResponse> {
        let id = req.id.clone();

        match req.method.as_str() {
            "initialize" => {
                let result = json!({
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {
                        "name": "oxydl-mcp",
                        "version": env!("CARGO_PKG_VERSION")
                    },
                    "capabilities": {
                        "tools": {}
                    }
                });
                Some(Self::success_resp(id, result))
            }

            "notifications/initialized" => {
                // Client acknowledged initialization; no response needed
                None
            }

            "ping" => Some(Self::success_resp(id, json!({}))),

            "tools/list" => {
                let tools = json!({
                    "tools": [
                        {
                            "name": "add_download",
                            "description": "Add a new download to OxyDL (supports HTTP/HTTPS, FTP, BitTorrent magnet, eD2K, HLS .m3u8, and DASH .mpd)",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "url": {
                                        "type": "string",
                                        "description": "Download URL, magnet link, eD2K URI, or stream manifest"
                                    },
                                    "filename": {
                                        "type": "string",
                                        "description": "Optional custom filename to save as"
                                    },
                                    "connections": {
                                        "type": "number",
                                        "description": "Number of concurrent parallel connections (1 to 32, default 8)"
                                    },
                                    "start_now": {
                                        "type": "boolean",
                                        "description": "Whether to begin downloading immediately (default true)"
                                    }
                                },
                                "required": ["url"]
                            }
                        },
                        {
                            "name": "list_downloads",
                            "description": "List all download tasks with their status, protocol, progress %, size, and speed",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "filter": {
                                        "type": "string",
                                        "description": "Optional filter: all, downloading, completed, paused, failed",
                                        "enum": ["all", "downloading", "completed", "paused", "failed"]
                                    }
                                }
                            }
                        },
                        {
                            "name": "pause_download",
                            "description": "Pause an active download by task ID or partial filename",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "task_id": {
                                        "type": "string",
                                        "description": "UUID of the task or matching filename"
                                    }
                                },
                                "required": ["task_id"]
                            }
                        },
                        {
                            "name": "resume_download",
                            "description": "Resume a paused or failed download task by ID or partial filename",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "task_id": {
                                        "type": "string",
                                        "description": "UUID of the task or matching filename"
                                    }
                                },
                                "required": ["task_id"]
                            }
                        },
                        {
                            "name": "cancel_download",
                            "description": "Delete and remove a download task from OxyDL",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "task_id": {
                                        "type": "string",
                                        "description": "UUID of the task to delete"
                                    },
                                    "delete_file": {
                                        "type": "boolean",
                                        "description": "Also delete partially downloaded file from disk"
                                    }
                                },
                                "required": ["task_id"]
                            }
                        },
                        {
                            "name": "get_download_details",
                            "description": "Get detailed chunk segment progress and metrics for a specific download",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "task_id": {
                                        "type": "string",
                                        "description": "UUID of the download task"
                                    }
                                },
                                "required": ["task_id"]
                            }
                        },
                        {
                            "name": "get_system_stats",
                            "description": "Get global download stats including total throughput speed, active task count, and storage info",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        },
                        {
                            "name": "set_speed_limit",
                            "description": "Configure or disable global download speed limiter",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "limit_kbps": {
                                        "type": "number",
                                        "description": "Maximum download rate in KB/s (pass 0 or null to uncap)"
                                    }
                                },
                                "required": ["limit_kbps"]
                            }
                        }
                    ]
                });
                Some(Self::success_resp(id, tools))
            }

            "tools/call" => {
                let params = req.params.unwrap_or_default();
                let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));

                let result = self.execute_tool(tool_name, args).await;
                Some(Self::success_resp(id, result))
            }

            _ => Some(Self::error_resp(id, -32601, format!("Method not found: {}", req.method))),
        }
    }

    async fn execute_tool(&self, name: &str, args: Value) -> Value {
        match name {
            "add_download" => {
                let url = match args.get("url").and_then(|v| v.as_str()) {
                    Some(u) if !u.trim().is_empty() => u.trim().to_string(),
                    _ => return Self::tool_error("Missing required parameter 'url'"),
                };

                let filename = args.get("filename")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| crate::util::file::extract_filename_from_url(&url));

                let connections = args.get("connections")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize)
                    .unwrap_or(8);

                let start_now = args.get("start_now")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                let save_dir = self.queue.settings.lock().await.default_save_dir.clone();
                let task_id = self.queue.add_task(url.clone(), filename.clone(), save_dir, connections, start_now).await;

                let proto = crate::manager::task::DownloadProtocol::detect(&url);
                Self::tool_success(&format!(
                    "Successfully added download task:\n- Task ID: {}\n- Protocol: {}\n- File: {}\n- Connections: {}\n- Status: {}",
                    task_id, proto.display_badge(), filename, connections, if start_now { "Downloading" } else { "Queued" }
                ))
            }

            "list_downloads" => {
                let filter = args.get("filter").and_then(|v| v.as_str()).unwrap_or("all");
                let tasks = self.queue.tasks.lock().await;

                if tasks.is_empty() {
                    return Self::tool_success("No downloads in queue.");
                }

                let mut out = String::new();
                out.push_str(&format!("Found {} download tasks:\n\n", tasks.len()));

                for (idx, t) in tasks.iter().enumerate() {
                    let matches_filter = match filter {
                        "downloading" => t.status == TaskStatus::Downloading,
                        "completed" => t.status == TaskStatus::Completed,
                        "paused" => t.status == TaskStatus::Paused,
                        "failed" => matches!(t.status, TaskStatus::Failed(_)),
                        _ => true,
                    };

                    if !matches_filter {
                        continue;
                    }

                    let pct = (t.fraction() * 100.0) as u32;
                    let size_str = match t.total_bytes {
                        Some(tot) => format!("{}/{}", format_bytes(t.downloaded_bytes), format_bytes(tot)),
                        None => format!("{}/Unknown", format_bytes(t.downloaded_bytes)),
                    };

                    out.push_str(&format!(
                        "{}. [{}] {} - {}\n   ID: {}\n   Progress: {}% ({}) | Speed: {} | ETA: {}\n   URL: {}\n\n",
                        idx + 1,
                        t.protocol.display_badge(),
                        t.file_name,
                        t.status.display_name(),
                        t.id,
                        pct,
                        size_str,
                        format_speed(t.speed_bps),
                        format_eta(t.eta_seconds),
                        t.url
                    ));
                }

                Self::tool_success(&out)
            }

            "pause_download" => {
                let target = match args.get("task_id").and_then(|v| v.as_str()) {
                    Some(s) => s.trim(),
                    None => return Self::tool_error("Missing parameter 'task_id'"),
                };

                let target_id = self.find_task_id(target).await;
                match target_id {
                    Some(id) => {
                        self.queue.pause_task(id).await;
                        Self::tool_success(&format!("Paused task {}", id))
                    }
                    None => Self::tool_error(&format!("Task '{}' not found", target)),
                }
            }

            "resume_download" => {
                let target = match args.get("task_id").and_then(|v| v.as_str()) {
                    Some(s) => s.trim(),
                    None => return Self::tool_error("Missing parameter 'task_id'"),
                };

                let target_id = self.find_task_id(target).await;
                match target_id {
                    Some(id) => {
                        self.queue.resume_task(id).await;
                        Self::tool_success(&format!("Resumed task {}", id))
                    }
                    None => Self::tool_error(&format!("Task '{}' not found", target)),
                }
            }

            "cancel_download" => {
                let target = match args.get("task_id").and_then(|v| v.as_str()) {
                    Some(s) => s.trim(),
                    None => return Self::tool_error("Missing parameter 'task_id'"),
                };

                let delete_file = args.get("delete_file").and_then(|v| v.as_bool()).unwrap_or(false);
                let target_id = self.find_task_id(target).await;
                match target_id {
                    Some(id) => {
                        self.queue.delete_task(id, delete_file).await;
                        Self::tool_success(&format!("Deleted task {} (file deleted: {})", id, delete_file))
                    }
                    None => Self::tool_error(&format!("Task '{}' not found", target)),
                }
            }

            "get_download_details" => {
                let target = match args.get("task_id").and_then(|v| v.as_str()) {
                    Some(s) => s.trim(),
                    None => return Self::tool_error("Missing parameter 'task_id'"),
                };

                let target_id = self.find_task_id(target).await;
                match target_id {
                    Some(id) => {
                        let tasks = self.queue.tasks.lock().await;
                        if let Some(t) = tasks.iter().find(|t| t.id == id) {
                            let details = json!({
                                "id": t.id.to_string(),
                                "url": t.url,
                                "file_name": t.file_name,
                                "protocol": t.protocol.display_badge(),
                                "save_path": t.save_path.display().to_string(),
                                "status": t.status.display_name(),
                                "downloaded_bytes": t.downloaded_bytes,
                                "total_bytes": t.total_bytes,
                                "progress_percent": format!("{:.1}%", t.fraction() * 100.0),
                                "speed": format_speed(t.speed_bps),
                                "eta": format_eta(t.eta_seconds),
                                "connections": t.num_connections,
                                "active_chunks": t.chunks.len(),
                            });
                            Self::tool_success(&serde_json::to_string_pretty(&details).unwrap_or_default())
                        } else {
                            Self::tool_error("Task not found")
                        }
                    }
                    None => Self::tool_error(&format!("Task '{}' not found", target)),
                }
            }

            "get_system_stats" => {
                let tasks = self.queue.tasks.lock().await;
                let active = tasks.iter().filter(|t| t.status == TaskStatus::Downloading).count();
                let completed = tasks.iter().filter(|t| t.status == TaskStatus::Completed).count();
                let total_speed: f64 = tasks.iter().filter(|t| t.status == TaskStatus::Downloading).map(|t| t.speed_bps).sum();
                let save_dir = self.queue.settings.lock().await.default_save_dir.clone();

                let stats = json!({
                    "app": "OxyDL",
                    "version": env!("CARGO_PKG_VERSION"),
                    "active_downloads": active,
                    "completed_downloads": completed,
                    "total_downloads": tasks.len(),
                    "total_speed": format_speed(total_speed),
                    "default_save_dir": save_dir.display().to_string(),
                });

                Self::tool_success(&serde_json::to_string_pretty(&stats).unwrap_or_default())
            }

            "set_speed_limit" => {
                let limit = args.get("limit_kbps").and_then(|v| v.as_u64());
                let mut settings = self.queue.settings.lock().await;
                settings.speed_limit_kbps = match limit {
                    Some(0) | None => None,
                    Some(n) => Some(n),
                };
                let _ = crate::manager::storage::Storage::save_settings(&settings);

                match settings.speed_limit_kbps {
                    Some(limit) => Self::tool_success(&format!("Global speed limit set to {} KB/s", limit)),
                    None => Self::tool_success("Global speed limiter disabled (unlimited throughput)"),
                }
            }

            _ => Self::tool_error(&format!("Unknown tool: {}", name)),
        }
    }

    async fn find_task_id(&self, target: &str) -> Option<Uuid> {
        if let Ok(id) = Uuid::parse_str(target) {
            return Some(id);
        }
        let tasks = self.queue.tasks.lock().await;
        let lower = target.to_lowercase();
        tasks.iter().find(|t| t.file_name.to_lowercase().contains(&lower)).map(|t| t.id)
    }

    fn tool_success(text: &str) -> Value {
        json!({
            "content": [
                {
                    "type": "text",
                    "text": text
                }
            ],
            "isError": false
        })
    }

    fn tool_error(text: &str) -> Value {
        json!({
            "content": [
                {
                    "type": "text",
                    "text": text
                }
            ],
            "isError": true
        })
    }

    fn success_resp(id: Option<Value>, result: Value) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error_resp(id: Option<Value>, code: i32, message: String) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(json!({
                "code": code,
                "message": message
            })),
        }
    }

    /// Run the MCP server over standard I/O (stdio) for AI desktop clients
    pub async fn run_stdio(self: Arc<Self>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin).lines();

        while let Some(line) = reader.next_line().await? {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }

            if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(line_trimmed) {
                if let Some(resp) = self.handle_request(req).await {
                    let resp_str = serde_json::to_string(&resp)?;
                    stdout.write_all(resp_str.as_bytes()).await?;
                    stdout.write_all(b"\n").await?;
                    stdout.flush().await?;
                }
            }
        }

        Ok(())
    }
}
