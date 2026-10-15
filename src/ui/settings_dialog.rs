//! Enhanced Tabbed Settings Dialog for OxyDL.
//! Provides categorized settings for General, Network, Storage, Protocols, and AI MCP integration.

use egui::{Color32, RichText, Window};
use crate::manager::storage::AppSettings;
use crate::util::file::get_app_data_dir;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    Connection,
    Storage,
    Protocols,
    Mcp,
}

pub struct SettingsDialogState {
    pub is_open: bool,
    pub current_tab: SettingsTab,
    pub settings: AppSettings,
    copied_mcp_config: bool,
}

impl SettingsDialogState {
    pub fn new(settings: AppSettings) -> Self {
        Self {
            is_open: false,
            current_tab: SettingsTab::General,
            settings,
            copied_mcp_config: false,
        }
    }

    pub fn open(&mut self, current: AppSettings) {
        self.settings = current;
        self.is_open = true;
        self.copied_mcp_config = false;
    }
}

pub enum SettingsDialogResult {
    None,
    Save(AppSettings),
    Cancel,
}

pub fn render_settings_dialog(
    ctx: &egui::Context,
    state: &mut SettingsDialogState,
) -> SettingsDialogResult {
    let mut result = SettingsDialogResult::None;

    if !state.is_open {
        return result;
    }

    let mut open = state.is_open;
    Window::new("⚙ OxyDL Preferences & Settings")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(620.0)
        .default_height(460.0)
        .show(ctx, |ui| {
            // Top tab selector bar
            ui.horizontal(|ui| {
                ui.selectable_value(&mut state.current_tab, SettingsTab::General, RichText::new("⚙ General").strong());
                ui.separator();
                ui.selectable_value(&mut state.current_tab, SettingsTab::Connection, RichText::new("🌐 Connection").strong());
                ui.separator();
                ui.selectable_value(&mut state.current_tab, SettingsTab::Storage, RichText::new("📁 Storage").strong());
                ui.separator();
                ui.selectable_value(&mut state.current_tab, SettingsTab::Protocols, RichText::new("🔀 Protocols").strong());
                ui.separator();
                ui.selectable_value(&mut state.current_tab, SettingsTab::Mcp, RichText::new("🤖 AI / MCP").strong());
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(10.0);

            // Tab Content Frame
            egui::ScrollArea::vertical()
                .max_height(340.0)
                .show(ui, |ui| {
                    match state.current_tab {
                        SettingsTab::General => render_general_tab(ui, &mut state.settings),
                        SettingsTab::Connection => render_connection_tab(ui, &mut state.settings),
                        SettingsTab::Storage => render_storage_tab(ui, &mut state.settings),
                        SettingsTab::Protocols => render_protocols_tab(ui, &mut state.settings),
                        SettingsTab::Mcp => render_mcp_tab(ui, &mut state.settings, &mut state.copied_mcp_config),
                    }
                });

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            // Footer with Save / Cancel actions
            ui.horizontal(|ui| {
                let save_btn = egui::Button::new(
                    RichText::new("💾 Save Preferences")
                        .strong()
                        .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(46, 204, 113));

                if ui.add(save_btn).clicked() {
                    state.is_open = false;
                    result = SettingsDialogResult::Save(state.settings.clone());
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Cancel").clicked() {
                        state.is_open = false;
                        result = SettingsDialogResult::Cancel;
                    }
                });
            });
        });

    state.is_open = open;
    result
}

fn render_general_tab(ui: &mut egui::Ui, s: &mut AppSettings) {
    ui.heading(RichText::new("Application & Workflow").size(15.0));
    ui.add_space(6.0);

    ui.checkbox(&mut s.auto_start_with_windows, "Launch OxyDL automatically on Windows startup");
    ui.checkbox(&mut s.minimize_to_tray, "Minimize to Windows System Notification Area (Tray) when closed");
    ui.checkbox(&mut s.play_sound_on_complete, "Play completion chime notification when downloads finish");

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);

    ui.heading(RichText::new("Clipboard Sniffer").size(15.0));
    ui.add_space(6.0);
    ui.checkbox(&mut s.monitor_clipboard, "Automatically detect copied download links (HTTP, FTP, Torrent, eD2K, HLS, DASH)");

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);

    ui.heading(RichText::new("Appearance & Storage Mode").size(15.0));
    ui.add_space(6.0);
    ui.checkbox(&mut s.dark_mode, "Enable High-Contrast Dark Visual Theme");

    let data_dir = get_app_data_dir();
    let is_portable = data_dir.ends_with("data");
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Storage Mode:").strong());
        if is_portable {
            ui.label(RichText::new("Portable Mode (Local ./data)").color(Color32::from_rgb(46, 204, 113)).strong());
        } else {
            ui.label(RichText::new("Standard System Mode (%APPDATA%)").color(Color32::from_rgb(52, 152, 219)));
        }
    });
    ui.label(RichText::new(format!("Data directory: {}", data_dir.display())).size(11.0).color(Color32::from_rgb(140, 145, 160)));
}

fn render_connection_tab(ui: &mut egui::Ui, s: &mut AppSettings) {
    ui.heading(RichText::new("Throughput & Concurrency").size(15.0));
    ui.add_space(6.0);

    // Speed limiter
    ui.horizontal(|ui| {
        let mut enabled = s.speed_limit_kbps.is_some();
        if ui.checkbox(&mut enabled, "Global Download Speed Limit:").changed() {
            s.speed_limit_kbps = if enabled { Some(1024) } else { None };
        }

        if let Some(ref mut limit) = s.speed_limit_kbps {
            ui.add(egui::DragValue::new(limit).range(64..=1_000_000).suffix(" KB/s"));
        } else {
            ui.label(RichText::new("(Unlimited throughput)").color(Color32::from_rgb(46, 204, 113)));
        }
    });

    ui.add_space(8.0);

    ui.horizontal(|ui| {
        ui.label("Maximum Concurrent Active Downloads:");
        egui::ComboBox::from_id_salt("settings_max_active")
            .selected_text(format!("{} Tasks", s.max_concurrent_downloads))
            .show_ui(ui, |ui| {
                for count in 1..=10 {
                    ui.selectable_value(&mut s.max_concurrent_downloads, count, format!("{} Tasks", count));
                }
            });
    });

    ui.add_space(6.0);

    ui.horizontal(|ui| {
        ui.label("Default Parallel Streams (Parts) per Download:");
        egui::ComboBox::from_id_salt("settings_default_conns")
            .selected_text(format!("{} Streams", s.default_connections))
            .show_ui(ui, |ui| {
                for &count in &[1, 2, 4, 8, 16, 24, 32] {
                    ui.selectable_value(&mut s.default_connections, count, format!("{} Streams", count));
                }
            });
    });

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);

    ui.heading(RichText::new("Network Socket Settings").size(15.0));
    ui.add_space(6.0);

    ui.horizontal(|ui| {
        ui.label("Connection Timeout:");
        ui.add(egui::DragValue::new(&mut s.socket_timeout_secs).range(5..=300).suffix(" seconds"));
    });

    ui.add_space(6.0);
    ui.label(RichText::new("HTTP User-Agent Header:").strong());
    ui.add(egui::TextEdit::singleline(&mut s.user_agent).desired_width(520.0));
}

fn render_storage_tab(ui: &mut egui::Ui, s: &mut AppSettings) {
    ui.heading(RichText::new("Download Directories").size(15.0));
    ui.add_space(6.0);

    ui.label(RichText::new("Default Save Destination:").strong());
    ui.horizontal(|ui| {
        let mut path_str = s.default_save_dir.to_string_lossy().to_string();
        ui.add(egui::TextEdit::singleline(&mut path_str).desired_width(420.0));

        if ui.button("📁 Browse...").clicked() {
            if let Some(folder) = rfd::FileDialog::new()
                .set_directory(&s.default_save_dir)
                .pick_folder()
            {
                s.default_save_dir = folder;
            }
        }
    });

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);

    ui.heading(RichText::new("Automated Categorization").size(15.0));
    ui.add_space(6.0);

    ui.checkbox(&mut s.categorize_subfolders, "Automatically create category subdirectories (Compressed, Documents, Music, Videos, Programs, Torrents)");
    ui.label(
        RichText::new("Files will automatically route to their dedicated folder based on mime-type and extension.")
            .size(11.0)
            .color(Color32::from_rgb(140, 145, 160)),
    );
}

fn render_protocols_tab(ui: &mut egui::Ui, s: &mut AppSettings) {
    ui.heading(RichText::new("Multi-Protocol Acceleration Engines").size(15.0));
    ui.add_space(8.0);

    // FTP
    ui.label(RichText::new("📁 File Transfer Protocol (FTP):").strong());
    ui.checkbox(&mut s.ftp_passive_mode, "Use Passive Data Mode (PASV - recommended for firewalls & NATs)");
    ui.add_space(8.0);

    // BitTorrent
    ui.label(RichText::new("🧲 BitTorrent & Magnet:").strong());
    ui.horizontal(|ui| {
        ui.label("Incoming Listen Port:");
        ui.add(egui::DragValue::new(&mut s.torrent_port).range(1024..=65535));
        ui.separator();
        ui.label("Max Peers per Torrent:");
        ui.add(egui::DragValue::new(&mut s.torrent_max_peers).range(5..=200));
    });
    ui.add_space(8.0);

    // eD2K
    ui.label(RichText::new("🐴 eDonkey2000 (eD2K):").strong());
    ui.horizontal(|ui| {
        ui.label("Client TCP Port:");
        ui.add(egui::DragValue::new(&mut s.ed2k_port).range(1024..=65535));
        ui.label(RichText::new("(Standard: 4662)").size(11.0).color(Color32::from_rgb(140, 145, 160)));
    });
    ui.add_space(8.0);

    // HLS & DASH
    ui.label(RichText::new("📺 Streaming Media (HLS .m3u8 & DASH .mpd):").strong());
    ui.horizontal(|ui| {
        ui.label("Parallel Media Segment Threads:");
        egui::ComboBox::from_id_salt("hls_parallel_segs")
            .selected_text(format!("{} Segments", s.hls_parallel_segments))
            .show_ui(ui, |ui| {
                for &cnt in &[1, 2, 4, 8, 12, 16] {
                    ui.selectable_value(&mut s.hls_parallel_segments, cnt, format!("{} Segments", cnt));
                }
            });
    });
}

fn render_mcp_tab(ui: &mut egui::Ui, s: &mut AppSettings, copied_mcp: &mut bool) {
    ui.heading(RichText::new("🤖 Model Context Protocol (MCP) Server").size(15.0));
    ui.add_space(6.0);

    ui.checkbox(&mut s.enable_mcp_server, "Enable Model Context Protocol (MCP) integration for AI assistants");
    ui.label(
        RichText::new("Allows AI coding tools and assistants (Claude Desktop, Cursor, Antigravity) to query active downloads, dispatch tasks, pause/resume, and check speeds.")
            .size(11.0)
            .color(Color32::from_rgb(140, 145, 160)),
    );

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);

    ui.heading(RichText::new("Local HTTP & Browser Integration API").size(15.0));
    ui.add_space(6.0);
    ui.checkbox(&mut s.enable_api_server, "Enable Local HTTP REST API (127.0.0.1)");

    ui.horizontal(|ui| {
        ui.label("API Listening Port:");
        ui.add(egui::DragValue::new(&mut s.api_port).range(1024..=65535));
        ui.label(RichText::new("(Default: 19532)").size(11.0).color(Color32::from_rgb(140, 145, 160)));
    });

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);

    ui.heading(RichText::new("Connect Claude Desktop / Cursor / Antigravity").size(15.0));
    ui.add_space(6.0);

    let config_snippet = r#"{
  "mcpServers": {
    "oxydl": {
      "command": "oxydl",
      "args": ["--mcp"]
    }
  }
}"#;

    ui.label("Paste this into your claude_desktop_config.json or cursor settings:");
    ui.add(
        egui::TextEdit::multiline(&mut config_snippet.to_string())
            .desired_rows(7)
            .desired_width(520.0)
            .font(egui::TextStyle::Monospace),
    );

    ui.add_space(6.0);
    if ui.button(if *copied_mcp { "✅ Copied to Clipboard!" } else { "📋 Copy MCP Configuration" }).clicked() {
        ui.output_mut(|o| o.copied_text = config_snippet.to_string());
        *copied_mcp = true;
    }
}
