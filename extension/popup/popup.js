// OxyDL Popup Script
const OXYDL_API_URL = "http://127.0.0.1:19532";

document.addEventListener("DOMContentLoaded", () => {
  const statusBadge = document.getElementById("status-badge");
  const appStatusText = document.getElementById("app-status-text");
  const activeDownloadsRow = document.getElementById("active-downloads-row");
  const activeDownloadsCount = document.getElementById("active-downloads-count");
  const interceptToggle = document.getElementById("intercept-toggle");
  const urlInput = document.getElementById("url-input");
  const sendBtn = document.getElementById("send-btn");
  const feedbackMsg = document.getElementById("feedback-msg");
  const currentTabBtn = document.getElementById("current-tab-btn");

  // Load auto-intercept toggle setting
  chrome.storage.local.get(["autoIntercept"], (result) => {
    if (result && result.autoIntercept !== undefined) {
      interceptToggle.checked = result.autoIntercept;
    } else {
      interceptToggle.checked = false;
    }
  });

  // Toggle listener
  interceptToggle.addEventListener("change", () => {
    chrome.storage.local.set({ autoIntercept: interceptToggle.checked }, () => {
      showFeedback(
        interceptToggle.checked
          ? "Automatic interception enabled"
          : "Automatic interception disabled",
        "success"
      );
    });
  });

  // Check desktop app connection
  async function checkAppStatus() {
    try {
      const response = await fetch(`${OXYDL_API_URL}/status`, {
        method: "GET",
        headers: { Accept: "application/json" },
        cache: "no-store",
      });

      if (response.ok) {
        const data = await response.json();
        statusBadge.textContent = "Connected";
        statusBadge.className = "badge badge-connected";
        appStatusText.textContent = `Online (v${data.version || "1.0"})`;
        activeDownloadsRow.style.display = "flex";
        activeDownloadsCount.textContent = data.active_downloads !== undefined ? data.active_downloads : 0;
      } else {
        markOffline();
      }
    } catch (e) {
      markOffline();
    }
  }

  function markOffline() {
    statusBadge.textContent = "Offline";
    statusBadge.className = "badge badge-disconnected";
    appStatusText.textContent = "Not Running";
    activeDownloadsRow.style.display = "none";
  }

  // Display feedback message
  let feedbackTimer = null;
  function showFeedback(text, type = "success") {
    clearTimeout(feedbackTimer);
    feedbackMsg.textContent = text;
    feedbackMsg.className = `feedback ${type}`;
    feedbackTimer = setTimeout(() => {
      feedbackMsg.textContent = "";
      feedbackMsg.className = "feedback";
    }, 4000);
  }

  // Send download URL to OxyDL
  async function sendDownload(url) {
    if (!url || (!url.startsWith("http://") && !url.startsWith("https://"))) {
      showFeedback("Please enter a valid HTTP or HTTPS URL", "error");
      return;
    }

    showFeedback("Sending to OxyDL...", "success");

    try {
      const response = await fetch(`${OXYDL_API_URL}/add`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          url: url,
          num_connections: 8,
          start_now: true,
        }),
      });

      if (response.ok) {
        const resData = await response.json();
        showFeedback(resData.message || "Download added to OxyDL!", "success");
        urlInput.value = "";
        checkAppStatus();
      } else {
        const errData = await response.json().catch(() => ({}));
        showFeedback(errData.error || "OxyDL rejected the request", "error");
      }
    } catch (err) {
      showFeedback("Could not connect to OxyDL. Make sure app is running.", "error");
    }
  }

  // Button & Enter key handlers
  sendBtn.addEventListener("click", () => {
    sendDownload(urlInput.value.trim());
  });

  urlInput.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      sendDownload(urlInput.value.trim());
    }
  });

  // Current tab button handler
  currentTabBtn.addEventListener("click", () => {
    if (chrome.tabs && chrome.tabs.query) {
      chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
        if (tabs && tabs[0] && tabs[0].url) {
          const tabUrl = tabs[0].url;
          if (tabUrl.startsWith("http://") || tabUrl.startsWith("https://")) {
            sendDownload(tabUrl);
          } else {
            showFeedback("Current tab is not a downloadable web page", "error");
          }
        } else {
          showFeedback("Could not determine current tab URL", "error");
        }
      });
    }
  });

  // Initial status check & periodic refresh
  checkAppStatus();
  setInterval(checkAppStatus, 5000);
});
