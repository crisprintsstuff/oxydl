<p align="center">
  <img src="assets/banner.png" alt="OxyDL - High Speed Download Manager" width="100%" />
</p>

<p align="center"><a href="CHANGELOG.md"><img src="https://img.shields.io/badge/changelog-view%20releases-
 ┃   blueviolet?style=flat-square&logo=git" alt="Changelog" /></a>
</p>

<p align="center">
  <a href="https://github.com/crisprintsstuff/oxydl/releases"><img src="https://img.shields.io/badge/release-v0.5.0-blue?style=flat-square&logo=github" alt="Release" /></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/language-Rust%201.80%2B-orange?style=flat-square&logo=rust" alt="Rust" /></a>
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20Linux-lightgrey?style=flat-square&logo=windows" alt="Platform" />
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-green?style=flat-square" alt="License" /></a>
  <a href="extension/"><img src="https://img.shields.io/badge/extension-Chrome%20%7C%20Firefox-informational?style=flat-square&logo=googlechrome" alt="Browser Extension" /></a>
</p>

> **OxyDL** is a lightweight, open-source, high-speed multi-connection download manager written in **100% Pure Rust**. Engineered as a clean, bloat-free alternative to **Internet Download Manager (IDM)**, **Free Download Manager (FDM)**, **JDownloader**, and **XDM**. Features dynamic multi-segment HTTP Range acceleration, real-time segment stream visualization, zero disk churn sparse pre-allocation, portable mode, native Windows installer wizard, universal Chrome/Firefox browser extension, scheduled queue timers, peak-hours speed limiter, YouTube & Vimeo video parser, and built-in FFmpeg media converter.

---

## ⚡ Highlights & Features

| Feature | OxyDL | Bloated Proprietary Managers |
| :--- | :---: | :---: |
| **Open Source & Free Forever** | ✅ Yes (MIT / Apache-2.0) | ❌ Paid / Adware / Trial Popups |
| **Pure Rust Native App** | ✅ Single ~6.2 MB `.exe` | ❌ 150+ MB Electron / C++ Bloat |
| **Multi-Part HTTP Range Streams** | ✅ 1 to 32 concurrent parts | ✅ |
| **Multi-Protocol Engine** | ✅ HTTP, FTP, Torrent, eD2K, HLS, DASH | ⚠️ Fragmented across plugins |
| **AI Integration (MCP Server)** | ✅ Built-in Model Context Protocol | ❌ None |
| **Automated Queue Calendar Timers** | ✅ Start/Stop times & Auto-Sleep/Shutdown | ⚠️ Complex / Paid |
| **Speed Scheduler** | ✅ Peak vs. Off-Peak bandwidth limits | ⚠️ Limited |
| **YouTube & Vimeo Video Parser** | ✅ Direct unencrypted CDN streams | ⚠️ Breakable / Paid add-on |
| **Media Converter & Audio Extractor** | ✅ FFmpeg integration (MP3, AAC, FLAC, MP4) | ❌ None or third-party bloat |
| **IDM-Style Segment Visualizer** | ✅ Real-time color-coded chunk bars | ✅ |
| **Instant Pause & Resume** | ✅ Byte-level `.oxydl.meta` tracking | ✅ |
| **Browser Extension** | ✅ Chrome, Edge, Firefox, Brave (Zero-bloat) | ⚠️ Proprietary / Adware / Popups |
| **Clipboard Link Sniffer** | ✅ Auto-detects copied URLs & video links | ✅ |
| **Portable Mode** | ✅ Runs from USB / any folder (`./data`) | ⚠️ Requires registry / AppData |
| **Zero Runtime Dependencies** | ✅ Self-contained, Pure Rustls TLS | ❌ OpenSSL / VC++ Runtimes required |

---

## 🚀 Key Architectural Capabilities

### 1. Dynamic Multi-Part Segmentation
Just like IDM's core speed acceleration mechanism, OxyDL queries remote servers for `Accept-Ranges: bytes` and `Content-Length`. If supported, it partitions files into $N$ parallel segments (configurable: 1, 2, 4, 8, 16, 24, 32 connections) and downloads each slice concurrently through non-blocking asynchronous I/O (`tokio` + `reqwest` + `rustls`).

### 2. Zero-Copy Sparse Pre-Allocation
Files are pre-allocated with their target size upfront. Each worker connection streams byte buffers directly into its dedicated file offset using atomic file seeking (`SeekFrom::Start`). This avoids the heavy disk churn and slow post-download file merging found in traditional download managers.

### 3. Reliable Pause & Resume
Every multi-part download generates a lightweight `.oxydl.meta` file recording the byte ranges, completed portions, and connection states. If a connection drops, your computer sleeps, or you pause a download, OxyDL seamlessly resumes right from the exact byte where it left off (with automatic fallback to legacy `.oxidm.meta` if present).

### 4. Iconic Segment Progress Visualizer
Inspired by IDM's legendary interface, selecting any download reveals the live segment canvas:
- Color-coded stream bars showing pending, active, and completed chunks.
- Live speed and progress metrics per connection thread.
- Moving-average speed smoothing and accurate ETA calculation.

### 5. Multi-Protocol Download Engine
OxyDL supports unified downloading across modern and legacy protocols with automatic protocol scheme detection and protocol badge indicators:
- **HTTP / HTTPS**: Multi-threaded dynamic byte-range partitioning (up to 32 parallel connections).
- **FTP**: Pure async FTP client supporting passive mode (`PASV`), binary mode (`TYPE I`), file size discovery (`SIZE`), and byte-level resume (`REST`).
- **BitTorrent & Magnet URIs**: Magnet link parsing (`magnet:?xt=urn:btih:...`), tracker querying, compact peer discovery, and block-level streaming.
- **eD2K (eDonkey2000)**: Native `ed2k://|file|...` URI support, 9,500 KB block partitioning, and sparse file allocation.
- **HLS Streaming**: HTTP Live Streaming (`.m3u8`) master and media playlist parsing with concurrent segment downloading and video assembly.
- **DASH Streaming**: Dynamic Adaptive Streaming over HTTP (`.mpd`) XML manifest parsing and segment template fetching.

### 6. Model Context Protocol (MCP) Server for AI Assistants
OxyDL includes a built-in MCP server compliant with Anthropic and Model Context Protocol specifications:
- **Stdio Mode**: Launch with `oxydl --mcp` for direct integration into Claude Desktop, Cursor, and Antigravity.
- **HTTP Endpoint**: Access via `POST http://127.0.0.1:19532/mcp` for local web and automation agents.
- **Available Tools**:
  - `add_download`: Add and schedule downloads across any supported protocol.
  - `list_downloads`: Query downloads filtered by status (`all`, `downloading`, `completed`, `paused`, `failed`).
  - `pause_download` & `resume_download`: Control specific tasks by ID or filename.
  - `cancel_download`: Abort tasks with optional on-disk file removal.
  - `get_download_details`: Granular piece, chunk, and protocol-level diagnostics.
  - `get_system_stats`: Global transfer speeds, active connection counts, and storage metrics.
  - `set_speed_limit`: Dynamically adjust global bandwidth throttling.

### 7. Automated Clipboard Monitoring
OxyDL includes a background clipboard watcher that identifies copied download links (`http://`, `https://`, `ftp://`, `magnet:`, `ed2k://`, `.m3u8`, `.mpd`) and renders a quick-action prompt banner to queue or download immediately.

### 8. True Portable Mode vs. System Install
- **Portable Mode**: Place `portable.txt` or a `data/` folder next to `oxydl.exe`. All task lists (`tasks.json`) and preferences (`settings.json`) are stored strictly inside `./data/`. Perfect for USB drives and zero-trace usage.
- **System Mode**: If no portable marker is present, OxyDL stores settings and task databases in standard OS directories (`%APPDATA%/OxyDL` on Windows, `~/.config/OxyDL` on Linux).

### 9. Chrome & Firefox Integration via Local REST API
OxyDL features a local, asynchronous HTTP REST API listening on `http://127.0.0.1:19532`:
- **Zero Brittle Native Messaging**: Avoids complex registry manifests and Native Messaging host headaches.
- **Universal Browser Support**: Works identically across Google Chrome, Mozilla Firefox, Microsoft Edge, Brave, Opera, and Vivaldi.
- **Auto-Interception**: Automatically catches browser downloads and redirects them to OxyDL.
- **Context Menus**: Right-click links, media, or web pages to send them instantly to OxyDL.

---

## 📂 Project Structure

```
oxydl/
├── Cargo.toml               # Project manifest & pure Rust dependencies
├── README.md                # Documentation & usage guide
├── assets/                  # Icons and visual assets
├── crates/
│   └── installer/           # Pure Rust native Windows installer (install_oxydl.exe)
├── extension/               # Chrome & Firefox Manifest V3 Extension
│   ├── manifest.json        # Universal MV3 browser manifest
│   ├── background.js        # Download interceptor & context menus
│   ├── popup/               # Modern dark popup UI (HTML/CSS/JS)
│   └── icons/               # High-res extension icons
├── installer/
│   ├── build_release.sh     # Release packaging script (Portable + Installer + Extension)
│   └── inno_setup.iss       # Inno Setup script fallback
└── src/
    ├── main.rs              # Application entry point & CLI flags (--mcp)
    ├── lib.rs               # Library root exposing engine, manager, and MCP
    ├── app.rs               # Main GUI state (eframe / egui) & event loop
    ├── api.rs               # Local HTTP API (127.0.0.1:19532) & /mcp endpoint
    ├── mcp.rs               # Model Context Protocol (MCP) server implementation
    ├── engine/              # Multi-protocol download engines
    │   ├── client.rs        # HTTP client, user-agent, headers & URL probe
    │   ├── downloader.rs    # Protocol dispatcher & worker coordinator
    │   ├── ftp.rs           # Pure async FTP engine (PASV, binary, resume)
    │   ├── torrent.rs       # BitTorrent engine (Magnet URIs & trackers)
    │   ├── ed2k.rs          # eDonkey2000 link parser & chunk stream
    │   ├── hls.rs           # HLS (.m3u8) streaming media parser & fetcher
    │   ├── dash.rs          # DASH (.mpd) XML manifest parser & downloader
    │   ├── speed.rs         # Moving-average speed calculation & throttling
    │   └── state.rs         # Chunk tracking & resume metadata (.oxydl.meta)
    ├── manager/             # Task queue & persistence
    │   ├── queue.rs         # Queue coordinator & concurrency scheduler
    │   ├── storage.rs       # tasks.json & settings.json persistence
    │   └── task.rs          # DownloadTask & DownloadProtocol definitions
    ├── ui/                  # Modular desktop GUI components
    │   ├── add_dialog.rs    # "Add New Download" modal with protocol detection
    │   ├── segment_bar.rs   # IDM multi-stream segment visualizer
    │   ├── settings_dialog.rs # 5-tab settings modal (General, Network, Protocols, AI/MCP)
    │   ├── sidebar.rs       # Category navigation & task count badges
    │   ├── table.rs         # Downloads list with protocol badges & context menu
    │   └── toolbar.rs       # Top action controls (Add, Resume, Pause, Speed)
    └── util/                # Formatters, file sanitization, & clipboard
        ├── clipboard.rs     # Clipboard multi-protocol URL detector
        ├── file.rs          # Category deduction & sparse file preallocation
        └── format.rs        # Byte, speed, and ETA formatting
```

---

## 🛠️ Building & Packaging

### Prerequisites
- [Rust](https://rustup.rs/) (Stable 1.80+)
- `cargo-xwin` (for compiling Windows `.exe` from Linux or Windows without Visual Studio)

### 1. Build Native Binary (Linux / Windows)
```bash
cargo build --release
```

### 2. Build Windows Portable `.exe` (Cross-compile from Linux)
```bash
cargo xwin build --target x86_64-pc-windows-msvc --release
```
The compiled binary will be located at:
`target/x86_64-pc-windows-msvc/release/oxydl.exe` (~5.6 MB standalone executable).

### 3. Generate Portable ZIP & Native Windows Installer
Run the automated release packaging script:
```bash
./installer/build_release.sh
```
This produces:
- **`dist/install_oxydl.exe`**: Standalone native Windows installer wizard (extracts payload, generates Desktop & Start Menu `.lnk` shortcuts, registers in Windows Apps & Features, and provisions `uninstall.exe`).
- **`dist/OxyDL-v0.1.0-Windows-x64-Portable.zip`**: Portable distribution with `portable.txt` enabled (stores data strictly in `./data`).
- **`dist/portable/oxydl.exe`**: Direct standalone portable binary.
- **`dist/oxydl-browser-extension.zip`**: Chrome & Firefox Manifest V3 browser integration package.

---

## 🌐 Browser Extension Setup (Chrome & Firefox)

The OxyDL browser extension connects directly to your running desktop application via `http://127.0.0.1:19532`.

### Google Chrome / Microsoft Edge / Brave
1. Go to `chrome://extensions` (or `edge://extensions`).
2. Toggle on **Developer mode** in the upper-right corner.
3. Click **Load unpacked** and select the `extension/` directory (or extract `oxydl-browser-extension.zip`).
4. Pin the OxyDL extension icon to your toolbar.

### Mozilla Firefox
1. Navigate to `about:debugging#/runtime/this-firefox`.
2. Click **Load Temporary Add-on...**.
3. Select `manifest.json` from the `extension/` folder.

---

## 🪟 Windows Installation & Setup System

### 1. GUI Installation Wizard (`install_oxydl.exe`)
Double-click `install_oxydl.exe` on any Windows PC:
- **Zero-Dependency**: 100% self-contained pure Rust installer (no admin elevation required).
- **Customizable**: Choose custom install location (defaults to `%LOCALAPPDATA%\Programs\OxyDL`).
- **Shortcuts**: Automatically creates Start Menu and Desktop shortcuts.
- **Windows Integration**: Registers in Windows "Installed apps" / "Add or Remove Programs" with icon, publisher, and version.

### 2. Silent / Automated Install (for Scripts & Winget)
Run in PowerShell or Command Prompt:
```cmd
install_oxydl.exe -y
```

### 3. Clean Uninstallation
To uninstall OxyDL:
- From Windows Settings: Go to **Installed apps** -> **OxyDL** -> **Uninstall**.
- Or run `uninstall.exe` directly from the install directory.
- Removes program files, Start Menu entries, Desktop shortcuts, and registry records.

---

## 📋 License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))
at your option.
