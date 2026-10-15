use oxidm::engine::state::{ChunkState, ChunkStatus, DownloadMeta};
use oxidm::manager::queue::QueueManager;
use oxidm::manager::task::TaskStatus;
use oxidm::util::file::{extract_filename_from_url, sanitize_filename, TaskCategory};

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
    let temp_dir = std::env::temp_dir().join(format!("oxidm_test_{}", uuid::Uuid::new_v4()));
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
