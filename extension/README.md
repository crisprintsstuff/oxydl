# Oxidm Browser Integration Extension

The official browser integration extension for **Oxidm**, the 100% free, open-source high-speed multi-connection download manager written in pure Rust.

Supports **Google Chrome**, **Mozilla Firefox**, **Microsoft Edge**, **Brave**, **Opera**, and any Chromium- or Gecko-based browser.

---

## ⚡ Features

- **One-Click Download Interception:** Automatically intercepts browser file downloads and transfers them seamlessly to Oxidm's multi-part acceleration engine.
- **Context Menus:**
  - Right-click any link -> **⚡ Download with Oxidm**
  - Right-click any image, video, or audio -> **⚡ Download Media with Oxidm**
  - Right-click anywhere on a webpage -> **⚡ Download Page with Oxidm**
- **Modern Dark Popup UI:**
  - Real-time connection status check with the Oxidm desktop app (`http://127.0.0.1:19532`).
  - Active download counter.
  - On/Off switch for automatic browser download interception.
  - Quick download input to paste and send any URL directly to Oxidm.
  - Quick action to download the current webpage link.
- **Zero Bloat & 100% Private:** No third-party trackers, no cloud dependencies, no analytics, no external servers. Communicates exclusively with your local Oxidm desktop instance.

---

## 🚀 Installation Guide

### Prerequisites
Make sure **Oxidm** is running on your machine. Oxidm automatically listens on `http://127.0.0.1:19532` to receive downloads from the extension.

---

### Option A: Google Chrome / Microsoft Edge / Brave / Opera

1. Open your browser and navigate to the extensions management page:
   - **Chrome / Brave:** `chrome://extensions`
   - **Edge:** `edge://extensions`
   - **Opera:** `opera://extensions`
2. Enable **Developer mode** (toggle located at the top-right in Chrome/Brave, or bottom-left in Edge).
3. Click the **Load unpacked** button.
4. Select the `extension/` folder in your Oxidm directory (or the extracted folder from `oxidm-browser-extension.zip`).
5. Pin the **Oxidm** icon to your browser toolbar for quick access!

---

### Option B: Mozilla Firefox

1. Open Firefox and navigate to `about:debugging#/runtime/this-firefox`.
2. Under the **Temporary Extensions** section, click **Load Temporary Add-on...**.
3. Select the `manifest.json` file inside the `extension/` folder.
4. The Oxidm Integration Module will be loaded and active immediately.

*(For permanent installation in Firefox Developer Edition / Nightly or unbranded builds, pack the folder into a `.zip` or `.xpi` and install via `about:addons` -> "Install Add-on From File").*

---

## 🛠️ How It Works

1. **Local REST API:** When Oxidm desktop runs, it binds a lightweight, asynchronous HTTP server to `127.0.0.1:19532`.
2. **CORS Enabled:** The API securely accepts requests with CORS headers from browser extensions.
3. **Dispatch:**
   - When a download is initiated or intercepted, the extension sends a `POST /add` JSON request containing the file URL and parameters.
   - Oxidm immediately begins multi-segment downloading with native speed acceleration!
