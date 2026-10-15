# Oxidm (Oxidized Download Manager) 🦀🚀

> A lightweight, open-source, high-speed multi-connection download manager written in **100% Pure Rust**. Designed as a clean, bloat-free, portable alternative to Internet Download Manager (IDM).

---

## ⚡ Highlights & Features

| Feature | Oxidm | Bloated Proprietary Managers |
| :--- | :---: | :---: |
| **Open Source & Free Forever** | ✅ Yes (MIT / Apache-2.0) | ❌ Paid / Adware / Trial Popups |
| **Pure Rust Native App** | ✅ Single ~5.6 MB `.exe` | ❌ 150+ MB Electron / C++ Bloat |
| **Multi-Part HTTP Range Streams** | ✅ 1 to 32 concurrent parts | ✅ |
| **IDM-Style Segment Visualizer** | ✅ Real-time color-coded chunk bars | ✅ |
| **Instant Pause & Resume** | ✅ Byte-level `.oxidm.meta` tracking | ✅ |
| **Speed Limiter** | ✅ Configurable per-task / global throttling | ✅ |
| **Clipboard Link Sniffer** | ✅ Auto-detects copied `http`/`https` URLs | ✅ |
| **Portable Mode** | ✅ Runs from USB / any folder (`./data`) | ⚠️ Requires registry / AppData |
| **Zero Runtime Dependencies** | ✅ Self-contained, Pure Rustls TLS | ❌ OpenSSL / VC++ Runtimes required |

---

## 🚀 Key Architectural Capabilities

### 1. Dynamic Multi-Part Segmentation
Just like IDM's core speed acceleration mechanism, Oxidm queries remote servers for `Accept-Ranges: bytes` and `Content-Length`. If supported, it partitions files into $N$ parallel segments (configurable: 1, 2, 4, 8, 16, 24, 32 connections) and downloads each slice concurrently through non-blocking asynchronous I/O (`tokio` + `reqwest` + `rustls`).

### 2. Zero-Copy Sparse Pre-Allocation
Files are pre-allocated with their target size upfront. Each worker connection streams byte buffers directly into its dedicated file offset using atomic file seeking (`SeekFrom::Start`). This avoids the heavy disk churn and slow post-download file merging found in traditional download managers.

### 3. Reliable Pause & Resume
Every multi-part download generates a lightweight `.oxidm.meta` file recording the byte ranges, completed portions, and connection states. If a connection drops, your computer sleeps, or you pause a download, Oxidm seamlessly resumes right from the exact byte where it left off.

### 4. Iconic Segment Progress Visualizer
Inspired by IDM's legendary interface, selecting any download reveals the live segment canvas:
- Color-coded stream bars showing pending, active, and completed chunks.
- Live speed and progress metrics per connection thread.
- Moving-average speed smoothing and accurate ETA calculation.

### 5. Automated Clipboard Monitoring
Oxidm includes a background clipboard watcher that identifies copied download links (`http://` or `https://`) and renders a quick-action prompt banner to queue or download immediately.

### 6. True Portable Mode vs. System Install
- **Portable Mode**: Place `portable.txt` or a `data/` folder next to `oxidm.exe`. All task lists (`tasks.json`) and preferences (`settings.json`) are stored strictly inside `./data/`. Perfect for USB drives and zero-trace usage.
- **System Mode**: If no portable marker is present, Oxidm stores settings and task databases in standard OS directories (`%APPDATA%/Oxidm` on Windows, `~/.config/Oxidm` on Linux).

---

## 📂 Project Structure

```
oxidm/
├── Cargo.toml               # Project manifest & pure Rust dependencies
├── README.md                # Documentation & usage guide
├── assets/                  # Icons and visual assets
├── installer/
│   ├── build_release.sh     # Release packaging script (Portable ZIP + Executable)
│   └── inno_setup.iss       # Inno Setup 1-click Windows installer script
└── src/
    ├── main.rs              # Application entry point & window bootstrapping
    ├── app.rs               # Main GUI state (eframe / egui) & event loop
    ├── engine/              # Multi-part download engine
    │   ├── client.rs        # HTTP client, user-agent, headers & URL probe
    │   ├── downloader.rs    # Concurrent chunk workers & stream writing
    │   ├── speed.rs         # Moving-average speed calculation & throttling
    │   └── state.rs         # Chunk tracking & resume metadata (.oxidm.meta)
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
`target/x86_64-pc-windows-msvc/release/oxidm.exe` (~5.6 MB standalone executable).

### 3. Generate Portable ZIP & Native Windows Installer
Run the automated release packaging script:
```bash
./installer/build_release.sh
```
This produces:
- **`dist/install_oxidm.exe`**: Standalone native Windows installer wizard (extracts payload, generates Desktop & Start Menu `.lnk` shortcuts, registers in Windows Apps & Features, and provisions `uninstall.exe`).
- **`dist/Oxidm-v0.1.0-Windows-x64-Portable.zip`**: Portable distribution with `portable.txt` enabled (stores data strictly in `./data`).
- **`dist/portable/oxidm.exe`**: Direct standalone portable binary.

---

## 🪟 Windows Installation & Setup System

### 1. GUI Installation Wizard (`install_oxidm.exe`)
Double-click `install_oxidm.exe` on any Windows PC:
- **Zero-Dependency**: 100% self-contained pure Rust installer (no admin elevation required).
- **Customizable**: Choose custom install location (defaults to `%LOCALAPPDATA%\Programs\Oxidm`).
- **Shortcuts**: Automatically creates Start Menu and Desktop shortcuts.
- **Windows Integration**: Registers in Windows "Installed apps" / "Add or Remove Programs" with icon, publisher, and version.

### 2. Silent / Automated Install (for Scripts & Winget)
Run in PowerShell or Command Prompt:
```cmd
install_oxidm.exe -y
```

### 3. Clean Uninstallation
To uninstall Oxidm:
- From Windows Settings: Go to **Installed apps** -> **Oxidm** -> **Uninstall**.
- Or run `uninstall.exe` directly from the install directory.
- Removes program files, Start Menu entries, Desktop shortcuts, and registry records.

---

## 📋 License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))
at your option.
