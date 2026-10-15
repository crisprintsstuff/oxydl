//! "Media Converter & Audio Extractor" modal dialog.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use egui::{Color32, RichText, Window};
use tokio::sync::{mpsc, oneshot};

use crate::engine::converter::{ConvertFormat, FfmpegDetector, FfmpegInfo, MediaConverter};

pub struct ConverterDialogState {
    pub is_open: bool,
    pub input_path: String,
    pub output_path: String,
    pub selected_format: ConvertFormat,
    pub ffmpeg_info: Option<FfmpegInfo>,
    pub is_converting: bool,
    pub progress: f32,
    pub status_text: String,
    pub error_msg: Option<String>,
    pub success_msg: Option<String>,
    cancel_token: Arc<AtomicBool>,
    progress_rx: Option<mpsc::Receiver<(f32, String)>>,
    result_rx: Option<oneshot::Receiver<Result<(), String>>>,
}

impl ConverterDialogState {
    pub fn new(custom_ffmpeg: Option<&Path>) -> Self {
        let ffmpeg_info = FfmpegDetector::find_ffmpeg(custom_ffmpeg);
        Self {
            is_open: false,
            input_path: String::new(),
            output_path: String::new(),
            selected_format: ConvertFormat::Mp3_320k,
            ffmpeg_info,
            is_converting: false,
            progress: 0.0,
            status_text: String::new(),
            error_msg: None,
            success_msg: None,
            cancel_token: Arc::new(AtomicBool::new(false)),
            progress_rx: None,
            result_rx: None,
        }
    }

    pub fn refresh_ffmpeg(&mut self, custom_ffmpeg: Option<&Path>) {
        self.ffmpeg_info = FfmpegDetector::find_ffmpeg(custom_ffmpeg);
    }

    pub fn open_with_file(&mut self, file_path: PathBuf, custom_ffmpeg: Option<&Path>) {
        self.refresh_ffmpeg(custom_ffmpeg);
        self.input_path = file_path.to_string_lossy().to_string();
        self.suggest_output_path();
        self.is_converting = false;
        self.progress = 0.0;
        self.status_text.clear();
        self.error_msg = None;
        self.success_msg = None;
        self.is_open = true;
    }

    pub fn open(&mut self, custom_ffmpeg: Option<&Path>) {
        self.refresh_ffmpeg(custom_ffmpeg);
        self.suggest_output_path();
        self.is_open = true;
    }

    fn suggest_output_path(&mut self) {
        if !self.input_path.is_empty() {
            let in_path = Path::new(&self.input_path);
            let ext = self.selected_format.extension();
            if let Some(stem) = in_path.file_stem().and_then(|s| s.to_str()) {
                let parent = in_path.parent().unwrap_or_else(|| Path::new(""));
                let out = parent.join(format!("{}.{}", stem, ext));
                self.output_path = out.to_string_lossy().to_string();
            }
        }
    }
}

pub fn render_converter_dialog(
    ctx: &egui::Context,
    state: &mut ConverterDialogState,
) {
    if !state.is_open {
        return;
    }

    // Poll background conversion progress
    if let Some(ref mut rx) = state.progress_rx {
        while let Ok((p, s)) = rx.try_recv() {
            state.progress = p;
            state.status_text = s;
        }
    }

    // Poll completion
    if let Some(ref mut rx) = state.result_rx {
        if let Ok(res) = rx.try_recv() {
            state.is_converting = false;
            state.progress_rx = None;
            state.result_rx = None;

            match res {
                Ok(()) => {
                    state.progress = 1.0;
                    state.success_msg = Some(format!("Successfully converted to: {}", state.output_path));
                    state.error_msg = None;
                }
                Err(e) => {
                    state.error_msg = Some(e);
                    state.success_msg = None;
                }
            }
        }
    }

    let mut open = state.is_open;
    Window::new("🎬 Media Converter & Audio Extractor")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(540.0)
        .show(ctx, |ui| {
            ui.add_space(4.0);

            // FFmpeg Status Banner
            ui.horizontal(|ui| {
                if let Some(ref ff) = state.ffmpeg_info {
                    ui.label(
                        RichText::new("✔ FFmpeg Detected:")
                            .color(Color32::from_rgb(46, 204, 113))
                            .strong(),
                    );
                    ui.label(
                        RichText::new(&ff.version)
                            .size(11.0)
                            .color(Color32::from_rgb(140, 145, 160)),
                    );
                } else {
                    ui.label(
                        RichText::new("⚠ FFmpeg Not Found")
                            .color(Color32::from_rgb(243, 156, 18))
                            .strong(),
                    );
                    if ui.button("Locate ffmpeg.exe...").clicked() {
                        if let Some(picked) = rfd::FileDialog::new()
                            .set_title("Locate FFmpeg Executable")
                            .pick_file()
                        {
                            state.refresh_ffmpeg(Some(&picked));
                        }
                    }
                }
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // Input File
            ui.label(RichText::new("Input Media File:").strong());
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut state.input_path)
                        .desired_width(430.0)
                        .hint_text("C:\\Videos\\downloaded_video.mp4"),
                );

                if ui.button("📁 Browse...").clicked() {
                    if let Some(file) = rfd::FileDialog::new()
                        .set_title("Select Audio/Video File to Convert")
                        .pick_file()
                    {
                        state.input_path = file.to_string_lossy().to_string();
                        state.suggest_output_path();
                    }
                }
            });

            ui.add_space(6.0);

            // Target Format Dropdown
            ui.label(RichText::new("Target Format / Output Quality:").strong());
            egui::ComboBox::from_id_salt("target_format_combo")
                .selected_text(state.selected_format.display_name())
                .width(430.0)
                .show_ui(ui, |ui| {
                    for fmt in ConvertFormat::ALL {
                        if ui.selectable_value(&mut state.selected_format, *fmt, fmt.display_name()).clicked() {
                            state.suggest_output_path();
                        }
                    }
                });

            ui.add_space(6.0);

            // Output File
            ui.label(RichText::new("Output Destination:").strong());
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut state.output_path)
                        .desired_width(430.0)
                        .hint_text("C:\\Music\\converted_song.mp3"),
                );

                if ui.button("📁 Browse...").clicked() {
                    let ext = state.selected_format.extension();
                    if let Some(file) = rfd::FileDialog::new()
                        .set_title("Select Converted File Destination")
                        .add_filter(ext, &[ext])
                        .save_file()
                    {
                        state.output_path = file.to_string_lossy().to_string();
                    }
                }
            });

            ui.add_space(8.0);

            // Progress / Status
            if state.is_converting {
                ui.label(RichText::new("Converting Media...").strong().color(Color32::from_rgb(52, 152, 219)));
                let pbar = egui::ProgressBar::new(state.progress)
                    .text(&state.status_text)
                    .animate(true);
                ui.add(pbar);
            }

            if let Some(ref success) = state.success_msg {
                ui.label(RichText::new(success).color(Color32::from_rgb(46, 204, 113)));
                ui.horizontal(|ui| {
                    if ui.button("📂 Open Converted File").clicked() {
                        let _ = open::that(&state.output_path);
                    }
                    if ui.button("📁 Open Folder").clicked() {
                        if let Some(parent) = Path::new(&state.output_path).parent() {
                            let _ = open::that(parent);
                        }
                    }
                });
            }

            if let Some(ref err) = state.error_msg {
                ui.label(RichText::new(format!("Error: {}", err)).color(Color32::from_rgb(231, 76, 60)));
            }

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);

            // Action Buttons
            ui.horizontal(|ui| {
                let can_convert = !state.is_converting
                    && state.ffmpeg_info.is_some()
                    && !state.input_path.trim().is_empty()
                    && !state.output_path.trim().is_empty();

                let convert_btn = ui.add_enabled(
                    can_convert,
                    egui::Button::new(
                        RichText::new("⚡ Start Conversion")
                            .strong()
                            .color(if can_convert { Color32::WHITE } else { Color32::GRAY }),
                    )
                    .fill(Color32::from_rgb(46, 204, 113)),
                );

                if convert_btn.clicked() {
                    state.is_converting = true;
                    state.progress = 0.0;
                    state.status_text = "Starting FFmpeg...".to_string();
                    state.error_msg = None;
                    state.success_msg = None;
                    state.cancel_token.store(false, Ordering::Relaxed);

                    let ffmpeg_bin = state.ffmpeg_info.as_ref().unwrap().binary_path.clone();
                    let input = PathBuf::from(state.input_path.trim());
                    let output = PathBuf::from(state.output_path.trim());
                    let format = state.selected_format;
                    let cancel_token = state.cancel_token.clone();

                    let (prog_tx, prog_rx) = mpsc::channel(32);
                    let (res_tx, res_rx) = oneshot::channel();
                    state.progress_rx = Some(prog_rx);
                    state.result_rx = Some(res_rx);

                    let ctx_clone = ctx.clone();
                    tokio::spawn(async move {
                        let res = MediaConverter::convert(
                            &ffmpeg_bin,
                            &input,
                            &output,
                            format,
                            cancel_token,
                            move |p, s| {
                                let _ = prog_tx.try_send((p, s));
                                ctx_clone.request_repaint();
                            },
                        ).await;

                        let mapped = res.map_err(|e| e.to_string());
                        let _ = res_tx.send(mapped);
                    });
                }

                if state.is_converting {
                    if ui.button("⏹ Cancel").clicked() {
                        state.cancel_token.store(true, Ordering::Relaxed);
                    }
                } else {
                    if ui.button("Close").clicked() {
                        state.is_open = false;
                    }
                }
            });
        });

    state.is_open = open;
}
