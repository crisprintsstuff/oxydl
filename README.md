<p align="center">
  <img src="assets/banner.png" alt="OxyDL - High Speed Download Manager" width="100%" />
</p>

<p align="center">
  <a href="https://github.com/crisprintsstuff/oxydl/releases"><img src="https://img.shields.io/badge/release-v0.1.0-blue?style=flat-square&logo=github" alt="Release" /></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/language-Rust%201.80%2B-orange?style=flat-square&logo=rust" alt="Rust" /></a>
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20Linux-lightgrey?style=flat-square&logo=windows" alt="Platform" />
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-green?style=flat-square" alt="License" /></a>
  <a href="extension/"><img src="https://img.shields.io/badge/extension-Chrome%20%7C%20Firefox-informational?style=flat-square&logo=googlechrome" alt="Browser Extension" /></a>
</p>

> **OxyDL** is a lightweight, open-source, high-speed multi-connection download manager written in **100% Pure Rust**. Engineered as a clean, bloat-free alternative to **Internet Download Manager (IDM)**, **Free Download Manager (FDM)**, **JDownloader**, and **XDM**. Features dynamic multi-segment HTTP Range acceleration, real-time segment stream visualization, zero disk churn sparse pre-allocation, portable mode, native Windows installer wizard, and universal Chrome/Firefox browser extension.

---

## ⚡ Highlights & Features

| Feature | OxyDL | Bloated Proprietary Managers |
| :--- | :---: | :---: |
| **Open Source & Free Forever** | ✅ Yes (MIT / Apache-2.0) | ❌ Paid / Adware / Trial Popups |
| **Pure Rust Native App** | ✅ Single ~5.6 MB `.exe` | ❌ 150+ MB Electron / C++ Bloat |
| **Multi-Part HTTP Range Streams** | ✅ 1 to 32 concurrent parts | ✅ |
| **IDM-Style Segment Visualizer** | ✅ Real-time color-coded chunk bars | ✅ |
| **Instant Pause & Resume** | ✅ Byte-level `.oxydl.meta` tracking | ✅ |
| **Speed Limiter** | ✅ Configurable per-task / global throttling | ✅ |
| **Browser Extension** | ✅ Chrome, Edge, Firefox, Brave (Zero-bloat) | ⚠️ Proprietary / Adware / Popups |
| **Clipboard Link Sniffer** | ✅ Auto-detects copied `http`/`https` URLs | ✅ |
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

### 5. Automated Clipboard Monitoring
OxyDL includes a background clipboard watcher that identifies copied download links (`http://` or `https://`) and renders a quick-action prompt banner to queue or download immediately.

### 6. True Portable Mode vs. System Install
- **Portable Mode**: Place `portable.txt` or a `data/` folder next to `oxydl.exe`. All task lists (`tasks.json`) and preferences (`settings.json`) are stored strictly inside `./data/`. Perfect for USB drives and zero-trace usage.
- **System Mode**: If no portable marker is present, OxyDL stores settings and task databases in standard OS directories (`%APPDATA%/OxyDL` on Windows, `~/.config/OxyDL` on Linux).

### 7. Chrome & Firefox Integration via Local REST API
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
    ├── main.rs              # Application entry point & window bootstrapping
    ├── app.rs               # Main GUI state (eframe / egui) & event loop
    ├── api.rs               # Local HTTP API (127.0.0.1:19532) for browser integration
    ├── engine/              # Multi-part download engine
    │   ├── client.rs        # HTTP client, user-agent, headers & URL probe
    │   ├── downloader.rs    # Concurrent chunk workers & stream writing
    │   ├── speed.rs         # Moving-average speed calculation & throttling
    │   └── state.rs         # Chunk tracking & resume metadata (.oxydl.meta)
    ├── manager/             # Task queue & persistence
    │   ├── queue.rs         # Queue coordinator & concurrency scheduler
    │   ├── storage.rs       # tasks.json & settings.json persistence
    │   └── task.rs          # DownloadTask definitions & status types
    ├── ui/                  # Modular desktop GUI components
    │   ├── add_dialog.rs    # "Add New Download" modal with probe preview
    │   ├── segment_bar.rs   # IDM multi-stream segment visualizer
    │   ├── settings_dialog.rs # Preferences modal
    │   ├── sidebar.rs       # Category navigation & task count badges
    │   ├── table.rs         # Downloads list, status badges, & context menu
    │   └── toolbar.rs       # Top action controls (Add, Resume, Pause, Speed)
    └── util/                # Formatters, file sanitization, & clipboard
        ├── clipboard.rs     # Clipboard URL detector
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
