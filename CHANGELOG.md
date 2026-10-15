# Changelog

All notable changes to the **OxyDL** project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Planned
- Cloud backup and sync for download queue states.
- Browser-side torrent magnet drag-and-drop handler.

---

## [0.5.0] - 2026-10-15

### Added
- **Automated Queue Calendar Timers**:
  - Background scheduler engine (`src/manager/scheduler.rs`) executing automated queue lifecycle evaluations.
  - Configurable 24-hour start and stop timers (`HH:MM`) with active day-of-week selection (`Mon`..`Sun`).
  - Optional one-shot calendar date targeting (`YYYY-MM-DD`).
  - Automated post-download actions upon queue completion: `PauseQueue`, `SleepComputer` (`rundll32.exe powrprof.dll` / `systemctl suspend`), and `ShutdownComputer`.

- **Peak-Hours Bandwidth Limiter (Speed Scheduler)**:
  - Dynamically throttles download speeds during defined business or peak usage hours (e.g., 09:00 to 18:00).
  - Configurable peak speed limits (KB/s) and off-peak speeds (unlimited or custom limit).
  - Time window support spanning midnight (e.g., 22:00 to 06:00).
  - Integrated into top toolbar via dedicated `⏰ Scheduler` status button and modal dialog (`src/ui/scheduler_dialog.rs`).

- **Direct Video Streaming Parser (YouTube & Vimeo)**:
  - Native video stream extractor (`src/engine/video.rs`) resolving direct CDN URLs without external dependencies.
  - **YouTube Parser**: Queries YouTube's InnerTube Android client API (`/youtubei/v1/player`) to retrieve unencrypted progressive formats (720p, 360p) and adaptive streams (1080p, 1440p, 4K, audio-only M4A/WebM) with metadata and thumbnails.
  - **Vimeo Parser**: Queries Vimeo player config API (`player.vimeo.com/video/{id}/config`) to retrieve direct progressive MP4 CDN URLs (1080p, 720p, 540p, 360p) and adaptive HLS master playlists.
  - Embedded stream selection chips in the "Add New Download" dialog with instantaneous quality selection.

- **Audio & Video Media Converter (FFmpeg Integration)**:
  - Built-in media transcoding and audio extraction suite (`src/engine/converter.rs` & `src/ui/converter_dialog.rs`).
  - Automatic multi-path discovery of `ffmpeg` binaries (custom user path, application directory, `./bin/`, PATH environment variable, Windows `%LOCALAPPDATA%` and `%ProgramFiles%`).
  - Supported output formats:
    - Audio: `MP3 (320 kbps & 192 kbps)`, `AAC (.m4a)`, `FLAC (Lossless)`, `WAV (PCM)`, and `Direct Stream Copy`.
    - Video: `MP4 (H.264 + AAC)`, `MKV (Fast Remux)`, and `WebM (VP9 + Opus)`.
  - Asynchronous background execution with real-time stderr parsing for live progress percentage, transcoding speed multiplier, and cancellation.
  - 1-click conversion accessible from the downloads list context menu (`🎬 Convert / Extract Audio...`) and main toolbar.

---

## [0.1.0] - 2026-10-15

### Added
- **Multi-Part Acceleration Engine**:
  - Pure Rust async engine powered by Tokio and Reqwest (`rustls-tls` with zero OpenSSL or C library dependencies).
  - Dynamic multi-stream byte-range segmentation (supporting 1 to 32 parallel connections).
  - Robust byte-level pause and resume recovery with binary `.oxydl.meta` checkpointing.
  - Sparse file allocation (`fallocate` on Linux, `SetFileValidData` on Windows) to prevent disk fragmentation.
  - Moving-average speed smoothing, ETA computation, and global bandwidth throttling.
  - IDM-style visual segment progress bar displaying live thread-level chunk progress.

- **Multi-Protocol Engine**:
  - **HTTP & HTTPS**: Dynamic range probing, redirect following, and Content-Disposition filename extraction.
  - **FTP**: Pure async FTP client supporting passive mode (`PASV`), binary mode (`TYPE I`), file size detection (`SIZE`), and byte-level resume (`REST`).
  - **BitTorrent**: Magnet URI parser (`magnet:?xt=urn:btih:...`), tracker scraping, compact peer discovery, and block streaming.
  - **eD2K (eDonkey2000)**: Native `ed2k://|file|...` URI support, 9,500 KB block partitioning, and sparse file allocation.
  - **HLS (HTTP Live Streaming)**: Master and media `.m3u8` playlist parser, concurrent segment downloader, and sequential TS stream stitching.
  - **DASH (Dynamic Adaptive Streaming)**: `.mpd` XML manifest parser, representation template resolution, and chunk downloader.
  - **Automatic Scheme Detection**: Seamless protocol identification and dedicated color badges in the downloads table and Add Download dialog.

- **Model Context Protocol (MCP) Server**:
  - Integrated JSON-RPC 2.0 MCP server for AI coding assistants (Claude Desktop, Cursor, Antigravity).
  - Stdio mode via `oxydl --mcp` flag for direct CLI pipes.
  - Local HTTP endpoint (`POST http://127.0.0.1:19532/mcp`) for local agent automation.
  - 8 core tools: `add_download`, `list_downloads`, `pause_download`, `resume_download`, `cancel_download`, `get_download_details`, `get_system_stats`, and `set_speed_limit`.

- **User Interface & Settings**:
  - Built with pure Rust `egui` and `eframe` (60 FPS, dark cyberpunk neon aesthetic).
  - Overhauled 5-tab settings window (`General`, `Connection`, `Storage`, `Protocols`, `AI / MCP`).
  - 1-click configuration snippet generator for `claude_desktop_config.json`.
  - Multi-category sidebar with live task counters (Compressed, Documents, Music, Videos, Programs, Other).
  - Filter search bar and full right-click context menu (Open File, Open Folder, Copy URL, Remove from List, Delete from Disk).
  - Background clipboard watcher for instant download prompts upon copying valid URLs.

- **Browser Extension (Chrome & Firefox)**:
  - Universal Manifest V3 extension in `extension/`.
  - Automatic download interception with whitelist/blacklist filtering.
  - Context menu items: "Download with OxyDL", "Download Link with OxyDL", "Download Video with OxyDL".
  - Popup status dashboard showing live transfer speeds and quick link addition.
  - Direct local communication over REST API (`http://127.0.0.1:19532`), eliminating Native Messaging host complexities.

- **Packaging & Portability**:
  - **Portable Mode**: Zero-install standalone archive (`OxyDL-v0.1.0-Windows-x64-Portable.zip`) operating out of `./data/`.
  - **Native Windows Installer**: Pure Rust installer (`install_oxydl.exe`) with embedded compressed payload (`oxydl.exe.gz`), auto-created Start Menu & Desktop shortcuts, clean uninstaller, and Windows Apps & Features registry integration.
  - Cross-compilation automated via `cargo-xwin` targeting `x86_64-pc-windows-msvc`.

### Changed
- Rebranded project from **Oxidm** to **OxyDL** with custom neon logo and visual assets.
- Optimized README badges and discovery keywords for search indexing.

---

[Unreleased]: https://github.com/crisprintsstuff/oxydl/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/crisprintsstuff/oxydl/compare/v0.1.0...v0.5.0
[0.1.0]: https://github.com/crisprintsstuff/oxydl/releases/tag/v0.1.0
