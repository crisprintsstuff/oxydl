use oxydl::engine::state::{ChunkState, ChunkStatus, DownloadMeta};
use oxydl::manager::queue::QueueManager;
use oxydl::manager::task::TaskStatus;
use oxydl::util::file::{extract_filename_from_url, sanitize_filename, TaskCategory};

#[test]
fn test_category_recognition() {
    assert_eq!(TaskCategory::from_filename("archive.tar.gz"), TaskCategory::Compressed);
    assert_eq!(TaskCategory::from_filename("report.pdf"), TaskCategory::Documents);
    assert_eq!(TaskCategory::from_filename("track01.flac"), TaskCategory::Music);
    assert_eq!(TaskCategory::from_filename("video.mp4"), TaskCategory::Videos);
    assert_eq!(TaskCategory::from_filename("setup.msi"), TaskCategory::Programs);
    assert_eq!(TaskCategory::from_filename("image.png"), TaskCategory::Other);
}

#[test]
fn test_url_filename_sanitization() {
    let url = "https://speed.cloudflare.com/__down?bytes=10000000";
    let extracted = extract_filename_from_url(url);
    assert_eq!(extracted, "__down");

    let dirty = "my/illegal*file?name<test>.bin";
    let clean = sanitize_filename(dirty);
    assert!(!clean.contains('/'));
    assert!(!clean.contains('*'));
    assert!(!clean.contains('?'));
    assert!(!clean.contains('<'));
}

#[test]
fn test_download_metadata_roundtrip() {
    let temp_dir = std::env::temp_dir().join(format!("oxydl_test_{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let target_file = temp_dir.join("testfile.dat");

    let meta = DownloadMeta {
        url: "https://example.com/testfile.dat".to_string(),
        file_name: "testfile.dat".to_string(),
        total_bytes: Some(10485760),
        supports_ranges: true,
        chunks: vec![
            ChunkState {
                index: 0,
                start: 0,
                end: 5242879,
                downloaded: 1024000,
                status: ChunkStatus::Downloading,
            },
            ChunkState {
                index: 1,
                start: 5242880,
                end: 10485759,
                downloaded: 0,
                status: ChunkStatus::Pending,
            },
        ],
    };

    // Save metadata
    assert!(meta.save_to_disk(&target_file).is_ok());

    // Load metadata
    let loaded = DownloadMeta::load_from_disk(&target_file);
    assert!(loaded.is_some());
    let loaded = loaded.unwrap();
    assert_eq!(loaded.url, meta.url);
    assert_eq!(loaded.total_bytes, meta.total_bytes);
    assert_eq!(loaded.chunks.len(), 2);
    assert_eq!(loaded.chunks[0].downloaded, 1024000);

    // Clean up
    DownloadMeta::remove_from_disk(&target_file);
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_queue_manager_lifecycle() {
    let queue = QueueManager::new();
    let temp_dir = std::env::temp_dir();

    // Add a paused task
    let task_id = queue.add_task(
        "https://example.com/software.zip".to_string(),
        "software.zip".to_string(),
        temp_dir.clone(),
        8,
        false, // do not start now
    ).await;

    // Verify task exists and is paused
    {
        let tasks = queue.tasks.lock().await;
        let task = tasks.iter().find(|t| t.id == task_id);
        assert!(task.is_some());
        let task = task.unwrap();
        assert_eq!(task.file_name, "software.zip");
        assert_eq!(task.status, TaskStatus::Paused);
        assert_eq!(task.category, TaskCategory::Compressed);
        assert_eq!(task.num_connections, 8);
    }

    // Delete task from queue
    queue.delete_task(task_id, false).await;

    {
        let tasks = queue.tasks.lock().await;
        let task = tasks.iter().find(|t| t.id == task_id);
        assert!(task.is_none());
    }
}

#[test]
fn test_protocol_detection() {
    use oxydl::manager::task::DownloadProtocol;

    assert_eq!(DownloadProtocol::detect("http://example.com/file.zip"), DownloadProtocol::Http);
    assert_eq!(DownloadProtocol::detect("https://example.com/file.zip"), DownloadProtocol::Http);
    assert_eq!(DownloadProtocol::detect("ftp://speedtest.tele2.net/1MB.zip"), DownloadProtocol::Ftp);
    assert_eq!(DownloadProtocol::detect("magnet:?xt=urn:btih:da39a3ee5e6b4b0d3255bfef95601890afd80709&dn=Ubuntu"), DownloadProtocol::Torrent);
    assert_eq!(DownloadProtocol::detect("https://example.com/arch.torrent"), DownloadProtocol::Torrent);
    assert_eq!(DownloadProtocol::detect("ed2k://|file|test.iso|1048576|D3B07384D113EDEC49EAA6238AD5FF00|/"), DownloadProtocol::Ed2k);
    assert_eq!(DownloadProtocol::detect("https://video.example.com/stream/index.m3u8"), DownloadProtocol::Hls);
    assert_eq!(DownloadProtocol::detect("https://video.example.com/dash/manifest.mpd"), DownloadProtocol::Dash);
}

#[test]
fn test_magnet_link_parsing() {
    use oxydl::engine::torrent::MagnetInfo;

    let magnet_uri = "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567&dn=MyVideo.mp4&xl=10485760";
    let parsed = MagnetInfo::parse(magnet_uri).expect("Failed to parse magnet link");

    assert_eq!(parsed.info_hash_hex, "0123456789abcdef0123456789abcdef01234567");
    assert_eq!(parsed.display_name, "MyVideo.mp4");
    assert_eq!(parsed.total_bytes, Some(10485760));
}

#[test]
fn test_ed2k_link_parsing() {
    use oxydl::engine::ed2k::Ed2kLink;

    let ed2k_uri = "ed2k://|file|sample_file.rar|52428800|31D6CFE0D16AE931B73C59D7E0C089C0|/";
    let parsed = Ed2kLink::parse(ed2k_uri).expect("Failed to parse eD2K link");

    assert_eq!(parsed.file_name, "sample_file.rar");
    assert_eq!(parsed.file_size, 52428800);
    assert_eq!(parsed.hash_hex, "31D6CFE0D16AE931B73C59D7E0C089C0");
}

#[tokio::test]
async fn test_mcp_server_initialize_and_tools() {
    use oxydl::mcp::{McpServer, JsonRpcRequest};
    use oxydl::manager::queue::QueueManager;

    let queue = QueueManager::new();
    let mcp = McpServer::new(queue);

    // Test initialize request
    let init_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::json!(1)),
        method: "initialize".to_string(),
        params: Some(serde_json::json!({})),
    };
    let init_resp = mcp.handle_request(init_req).await.expect("Expected response");
    assert!(init_resp.result.is_some());
    let res = init_resp.result.unwrap();
    assert_eq!(res["serverInfo"]["name"], "oxydl-mcp");

    // Test tools/list request
    let list_tools_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::json!(2)),
        method: "tools/list".to_string(),
        params: Some(serde_json::json!({})),
    };
    let tools_resp = mcp.handle_request(list_tools_req).await.expect("Expected response");
    let res = tools_resp.result.unwrap();
    let tools = res["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 8);

    // Test tools/call list_downloads
    let call_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::json!(3)),
        method: "tools/call".to_string(),
        params: Some(serde_json::json!({
            "name": "list_downloads",
            "arguments": {
                "status": "all"
            }
        })),
    };
    let call_resp = mcp.handle_request(call_req).await.expect("Expected response");
    let res = call_resp.result.unwrap();
    assert_eq!(res["isError"], false);
}

