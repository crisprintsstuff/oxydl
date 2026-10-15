//! "Add New Download" modal dialog.

use std::path::PathBuf;
use egui::{Color32, RichText, Window};

use crate::engine::client::{create_http_client, probe_url, ProbeResult};
use crate::engine::ed2k::Ed2kLink;
use crate::engine::torrent::MagnetInfo;
use crate::engine::video::{VideoExtractor, VideoInfo};
use crate::manager::task::DownloadProtocol;
use crate::util::file::{extract_filename_from_url, sanitize_filename, TaskCategory};
use crate::util::format::format_bytes;

pub struct AddDialogState {
    pub is_open: bool,
    pub url: String,
    pub file_name: String,
    pub save_dir: PathBuf,
    pub num_connections: usize,
    pub category: TaskCategory,
    pub is_probing: bool,
    pub probe_result: Option<ProbeResult>,
    pub error_msg: Option<String>,
    pub probe_pending: Option<tokio::sync::oneshot::Receiver<Result<ProbeResult, String>>>,
    pub video_info: Option<VideoInfo>,
    pub is_extracting_video: bool,
    pub video_pending: Option<tokio::sync::oneshot::Receiver<Result<VideoInfo, String>>>,
}

impl AddDialogState {
    pub fn new(default_save_dir: PathBuf, default_connections: usize) -> Self {
        Self {
            is_open: false,
            url: String::new(),
            file_name: String::new(),
            save_dir: default_save_dir,
            num_connections: default_connections,
            category: TaskCategory::Other,
            is_probing: false,
            probe_result: None,
            error_msg: None,
            probe_pending: None,
            video_info: None,
            is_extracting_video: false,
            video_pending: None,
        }
    }

    pub fn open_with_url(&mut self, url: String) {
        self.url = url.clone();
        self.file_name = extract_filename_from_url(&url);
        self.category = TaskCategory::from_filename(&self.file_name);
        self.probe_result = None;
        self.error_msg = None;
        self.probe_pending = None;
        self.is_probing = false;
        self.video_info = None;
        self.is_extracting_video = false;
        self.video_pending = None;
        self.is_open = true;
    }
}

pub enum AddDialogResult {
    None,
    DownloadNow {
        url: String,
        file_name: String,
        save_dir: PathBuf,
        num_connections: usize,
    },
    QueueOnly {
        url: String,
        file_name: String,
        save_dir: PathBuf,
        num_connections: usize,
    },
    Cancel,
}

pub fn render_add_dialog(
    ctx: &egui::Context,
    state: &mut AddDialogState,
) -> AddDialogResult {
    let mut result = AddDialogResult::None;

    if !state.is_open {
        return result;
    }

    // Check if background probe completed
    if let Some(ref mut rx) = state.probe_pending {
        if let Ok(res) = rx.try_recv() {
            match res {
                Ok(probe) => {
                    state.file_name = probe.suggested_filename.clone();
                    state.category = TaskCategory::from_filename(&state.file_name);
                    state.probe_result = Some(probe);
                    state.error_msg = None;
                }
                Err(err) => {
                    state.error_msg = Some(err);
                }
            }
            state.is_probing = false;
            state.probe_pending = None;
        }
    }

    // Check if background video extraction completed
    if let Some(ref mut rx) = state.video_pending {
        if let Ok(res) = rx.try_recv() {
            state.is_extracting_video = false;
            state.video_pending = None;
            match res {
                Ok(v_info) => {
                    state.file_name = format!("{}.mp4", sanitize_filename(&v_info.title));
                    state.category = TaskCategory::Videos;
                    state.error_msg = None;
                    state.video_info = Some(v_info);
                }
                Err(err) => {
                    state.error_msg = Some(err);
                }
            }
        }
    }

    let mut open = state.is_open;
    Window::new("➕ Enter New Download Address")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(520.0)
        .show(ctx, |ui| {
            ui.add_space(4.0);

            // URL Row
            ui.label(RichText::new("Address (URL):").strong());
            ui.horizontal(|ui| {
                let url_resp = ui.add(
                    egui::TextEdit::singleline(&mut state.url)
                        .desired_width(430.0)
                        .hint_text("https://example.com/file.zip"),
                );

                if url_resp.changed() {
                    state.file_name = extract_filename_from_url(&state.url);
                    state.category = TaskCategory::from_filename(&state.file_name);
                }

                if ui.button("📋 Paste").clicked() {
                    if let Ok(mut cb) = arboard::Clipboard::new() {
                        if let Ok(text) = cb.get_text() {
                            let text = text.trim();
                            if !text.is_empty() {
                                state.url = text.to_string();
                                state.file_name = extract_filename_from_url(&state.url);
                                state.category = TaskCategory::from_filename(&state.file_name);
                            }
                        }
                    }
                }
            });

            // Protocol Indicator
            let proto = DownloadProtocol::detect(&state.url);
            let (r, g, b) = proto.rgb();
            let proto_color = Color32::from_rgb(r, g, b);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Protocol:").size(12.0));
                ui.label(
                    RichText::new(format!("{} {}", proto.icon(), proto.display_badge()))
                        .color(proto_color)
                        .strong()
                        .size(12.0),
                );
            });

            ui.add_space(6.0);

            // Probe URL button & info
            ui.horizontal(|ui| {
                let probe_btn_label = if state.is_probing {
                    "⏳ Inspecting..."
                } else {
                    "🔍 Check / Inspect URL"
                };

                if ui.button(probe_btn_label).clicked() && !state.url.trim().is_empty() && !state.is_probing {
                    let url = state.url.clone();
                    let proto = DownloadProtocol::detect(&url);

                    match proto {
                        DownloadProtocol::Torrent => {
                            match MagnetInfo::parse(&url) {
                                Ok(magnet) => {
                                    state.file_name = magnet.display_name.clone();
                                    state.category = TaskCategory::from_filename(&state.file_name);
                                    state.probe_result = Some(ProbeResult {
                                        url: url.clone(),
                                        suggested_filename: magnet.display_name,
                                        total_bytes: magnet.total_bytes,
                                        supports_ranges: false,
                                        content_type: Some("application/x-bittorrent".to_string()),
                                    });
                                    state.error_msg = None;
                                }
                                Err(e) => {
                                    state.error_msg = Some(format!("BitTorrent error: {}", e));
                                }
                            }
                        }
                        DownloadProtocol::Ed2k => {
                            match Ed2kLink::parse(&url) {
                                Ok(ed2k) => {
                                    state.file_name = ed2k.file_name.clone();
                                    state.category = TaskCategory::from_filename(&state.file_name);
                                    state.probe_result = Some(ProbeResult {
                                        url: url.clone(),
                                        suggested_filename: ed2k.file_name,
                                        total_bytes: Some(ed2k.file_size),
                                        supports_ranges: false,
                                        content_type: Some("application/x-ed2k".to_string()),
                                    });
                                    state.error_msg = None;
                                }
                                Err(e) => {
                                    state.error_msg = Some(format!("eD2K error: {}", e));
                                }
                            }
                        }
                        DownloadProtocol::Http | DownloadProtocol::Ftp | DownloadProtocol::Hls | DownloadProtocol::Dash => {
                            state.is_probing = true;
                            state.error_msg = None;

                            let (tx, rx) = tokio::sync::oneshot::channel();
                            state.probe_pending = Some(rx);

                            let ctx_clone = ctx.clone();
                            tokio::spawn(async move {
                                if let Ok(client) = create_http_client() {
                                    let probe = probe_url(&client, &url).await;
                                    let _ = tx.send(probe);
                                } else {
                                    let _ = tx.send(Err("Failed to initialize client".to_string()));
                                }
                                ctx_clone.request_repaint();
                            });
                        }
                    }
                }

                if let Some(ref probe) = state.probe_result {
                    if let Some(sz) = probe.total_bytes {
                        ui.label(
                            RichText::new(format!("Size: {}", format_bytes(sz)))
                                .color(Color32::from_rgb(46, 204, 113))
                                .strong(),
                        );
                    }
                    if probe.supports_ranges {
                        ui.label(
                            RichText::new("⚡ Multi-part range supported")
                                .color(Color32::from_rgb(52, 152, 219)),
                        );
                    }
                }
            });

            // Video Stream Extraction for YouTube and Vimeo
            let is_video_stream = VideoExtractor::is_supported(&state.url);
            if is_video_stream {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let extract_label = if state.is_extracting_video {
                        "⏳ Fetching Video Streams..."
                    } else {
                        "🎬 Fetch Video Streams (YouTube / Vimeo)"
                    };

                    if ui.button(extract_label).clicked() && !state.is_extracting_video {
                        state.is_extracting_video = true;
                        state.error_msg = None;
                        let (tx, rx) = tokio::sync::oneshot::channel();
                        state.video_pending = Some(rx);
                        let url_clone = state.url.clone();
                        let ctx_clone = ctx.clone();
                        tokio::spawn(async move {
                            let res = VideoExtractor::extract(&url_clone).await.map_err(|e| e.to_string());
                            let _ = tx.send(res);
                            ctx_clone.request_repaint();
                        });
                    }
                });
            }

            if let Some(ref v_info) = state.video_info {
                ui.add_space(4.0);
                ui.group(|ui| {
                    ui.label(RichText::new(format!("{} {}", v_info.provider.icon(), v_info.title)).strong().color(Color32::from_rgb(52, 152, 219)));
                    ui.label(format!("Creator: {} • Duration: {}m", v_info.author, v_info.duration_secs.unwrap_or(0) / 60));
                    ui.add_space(4.0);
                    ui.label(RichText::new("Select Stream Quality:").strong());
                    ui.horizontal_wrapped(|ui| {
                        for stream in &v_info.streams {
                            if ui.button(&stream.quality_label).clicked() {
                                state.url = stream.direct_url.clone();
                                state.file_name = format!("{}.{}", sanitize_filename(&v_info.title), stream.extension);
                                state.category = if stream.is_audio_only { TaskCategory::Music } else { TaskCategory::Videos };
                                state.probe_result = Some(ProbeResult {
                                    url: stream.direct_url.clone(),
                                    suggested_filename: state.file_name.clone(),
                                    total_bytes: stream.file_size,
                                    supports_ranges: true,
                                    content_type: Some(if stream.is_audio_only { "audio/mp4".to_string() } else { "video/mp4".to_string() }),
                                });
                            }
                        }
                    });
                });
            }

            if let Some(ref err) = state.error_msg {
                ui.label(RichText::new(err).color(Color32::from_rgb(231, 76, 60)));
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // File Name Row
            ui.label(RichText::new("File Name:").strong());
            let name_resp = ui.text_edit_singleline(&mut state.file_name);
            if name_resp.changed() {
                state.category = TaskCategory::from_filename(&state.file_name);
            }

            ui.add_space(6.0);

            // Save Directory Row
            ui.label(RichText::new("Save To Directory:").strong());
            ui.horizontal(|ui| {
                let mut dir_str = state.save_dir.to_string_lossy().to_string();
                ui.add(
                    egui::TextEdit::singleline(&mut dir_str)
                        .desired_width(410.0),
                );

                if ui.button("📁 Browse...").clicked() {
                    if let Some(folder) = rfd::FileDialog::new()
                        .set_directory(&state.save_dir)
                        .pick_folder()
                    {
                        state.save_dir = folder;
                    }
                }
            });

            ui.add_space(6.0);

            // Connections & Category Row
            ui.horizontal(|ui| {
                ui.label("Connections (Parts):");
                egui::ComboBox::from_id_salt("conn_select")
                    .selected_text(format!("{} Parts", state.num_connections))
                    .show_ui(ui, |ui| {
                        for &c in &[1, 2, 4, 8, 16, 24, 32] {
                            ui.selectable_value(&mut state.num_connections, c, format!("{} Parts", c));
                        }
                    });

                ui.add_space(16.0);
                ui.label("Category:");
                egui::ComboBox::from_id_salt("cat_select")
                    .selected_text(format!("{} {}", state.category.icon(), state.category.display_name()))
                    .show_ui(ui, |ui| {
                        for cat in &[
                            TaskCategory::Compressed,
                            TaskCategory::Documents,
                            TaskCategory::Music,
                            TaskCategory::Videos,
                            TaskCategory::Programs,
                            TaskCategory::Other,
                        ] {
                            ui.selectable_value(&mut state.category, *cat, format!("{} {}", cat.icon(), cat.display_name()));
                        }
                    });
            });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);

            // Action Buttons
            ui.horizontal(|ui| {
                let valid = !state.url.trim().is_empty() && !state.file_name.trim().is_empty();

                let dl_btn = egui::Button::new(
                    RichText::new("🚀 Download Now")
                        .strong()
                        .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(46, 204, 113)); // Emerald

                if ui.add_enabled(valid, dl_btn).clicked() {
                    state.is_open = false;
                    result = AddDialogResult::DownloadNow {
                        url: state.url.trim().to_string(),
                        file_name: state.file_name.trim().to_string(),
                        save_dir: state.save_dir.clone(),
                        num_connections: state.num_connections,
                    };
                }

                let queue_btn = egui::Button::new("📥 Add to Queue");
                if ui.add_enabled(valid, queue_btn).clicked() {
                    state.is_open = false;
                    result = AddDialogResult::QueueOnly {
                        url: state.url.trim().to_string(),
                        file_name: state.file_name.trim().to_string(),
                        save_dir: state.save_dir.clone(),
                        num_connections: state.num_connections,
                    };
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Cancel").clicked() {
                        state.is_open = false;
                        result = AddDialogResult::Cancel;
                    }
                });
            });
        });

    state.is_open = open;
    result
}
