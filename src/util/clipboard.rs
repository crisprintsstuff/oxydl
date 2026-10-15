//! Clipboard monitoring utility to catch download links.

use std::sync::{Arc, Mutex};
use arboard::Clipboard;

#[allow(dead_code)]
pub struct ClipboardMonitor {
    clipboard: Option<Arc<Mutex<Clipboard>>>,
    last_url: Arc<Mutex<Option<String>>>,
}

impl ClipboardMonitor {
    pub fn new() -> Self {
        let clipboard = Clipboard::new().ok().map(|cb| Arc::new(Mutex::new(cb)));
        Self {
            clipboard,
            last_url: Arc::new(Mutex::new(None)),
        }
    }

    /// Check if the clipboard contains a new valid HTTP/HTTPS link.
    /// If so, returns the new URL and updates internal state so it won't trigger again.
    pub fn poll_new_url(&self) -> Option<String> {
        let cb_arc = self.clipboard.as_ref()?;
        let mut cb = cb_arc.lock().ok()?;

        if let Ok(text) = cb.get_text() {
            let trimmed = text.trim();
            if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
                if let Ok(parsed) = url::Url::parse(trimmed) {
                    if parsed.host_str().is_some() {
                        let mut last = self.last_url.lock().ok()?;
                        if last.as_deref() != Some(trimmed) {
                            *last = Some(trimmed.to_string());
                            return Some(trimmed.to_string());
                        }
                    }
                }
            }
        }
        None
    }

    /// Clear last seen URL so the current clipboard can be detected again if re-copied.
    pub fn reset_last(&self) {
        if let Ok(mut last) = self.last_url.lock() {
            *last = None;
        }
    }
}
