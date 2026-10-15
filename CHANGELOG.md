# Changelog

All notable changes to the **OxyDL** project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Planned
- Scheduled download queues with start/stop calendar timers.
- Speed scheduler for bandwidth-limiting during peak hours.
- Direct video streaming parser for YouTube and Vimeo video streams.
- Audio and video media converter (FFmpeg integration).

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

[Unreleased]: https://github.com/crisprintsstuff/oxydl/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/crisprintsstuff/oxydl/releases/tag/v0.1.0
