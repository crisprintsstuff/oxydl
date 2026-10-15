// Oxidm Background Service Worker for Chrome & Firefox
const OXIDM_API_URL = "http://127.0.0.1:19532";

// Initialize Context Menus upon installation
chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({
    id: "oxidm-download-link",
    title: "⚡ Download with Oxidm",
    contexts: ["link"],
  });

  chrome.contextMenus.create({
    id: "oxidm-download-media",
    title: "⚡ Download Media with Oxidm",
    contexts: ["image", "video", "audio"],
  });

  chrome.contextMenus.create({
    id: "oxidm-download-page",
    title: "⚡ Download Page with Oxidm",
    contexts: ["page"],
  });

  // Set default settings
  chrome.storage.local.get(["autoIntercept", "minFileSizeMB"], (res) => {
    if (res.autoIntercept === undefined) {
      chrome.storage.local.set({ autoIntercept: false, minFileSizeMB: 1 });
    }
  });
});

// Context Menu Click Handler
chrome.contextMenus.onClicked.addListener((info, tab) => {
  let targetUrl = info.linkUrl || info.srcUrl || info.pageUrl;
  if (!targetUrl) return;

  let suggestedName = null;
  if (info.srcUrl) {
    try {
      let pathSegments = new URL(info.srcUrl).pathname.split("/");
      suggestedName = pathSegments[pathSegments.length - 1];
    } catch (e) {}
  }

  sendToOxidm(targetUrl, suggestedName, tab ? tab.url : null);
});

// Automatic Browser Download Interceptor
chrome.downloads.onCreated.addListener((downloadItem) => {
  chrome.storage.local.get(["autoIntercept"], (settings) => {
    if (!settings.autoIntercept) return;

    let url = downloadItem.url || downloadItem.finalUrl;
    if (!url || (!url.startsWith("http://") && !url.startsWith("https://"))) {
      return; // Skip data: and blob: URLs
    }

    // Cancel native browser download
    chrome.downloads.cancel(downloadItem.id, () => {
      chrome.downloads.erase({ id: downloadItem.id }, () => {});
    });

    // Extract filename from browser item
    let filename = downloadItem.filename ? downloadItem.filename.split(/[/\\]/).pop() : null;
    let referrer = downloadItem.referrer || null;

    sendToOxidm(url, filename, referrer, true);
  });
});

// Communication with Oxidm Desktop Application via Local HTTP API
async function sendToOxidm(url, filename, referrer, intercepted = false) {
  try {
    const payload = {
      url: url,
      file_name: filename || null,
      start_now: true,
      num_connections: 8,
    };

    const response = await fetch(`${OXIDM_API_URL}/add`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });

    if (response.ok) {
      const data = await response.json();
      showNotification(
        "🚀 Oxidm: Download Added",
        filename ? `Downloading: ${filename}` : `Transferring URL to Oxidm...`
      );
    } else {
      showNotification(
        "⚠️ Oxidm Error",
        "Oxidm rejected the download request. Check application logs."
      );
    }
  } catch (error) {
    showNotification(
      "⚠️ Oxidm Not Running",
      "Could not connect to Oxidm on 127.0.0.1:19532. Please launch Oxidm on your computer."
    );
  }
}

// Notification Helper
function showNotification(title, message) {
  if (chrome.notifications && chrome.notifications.create) {
    chrome.notifications.create({
      type: "basic",
      iconUrl: "icons/icon128.png",
      title: title,
      message: message,
      priority: 1,
    });
  }
}
