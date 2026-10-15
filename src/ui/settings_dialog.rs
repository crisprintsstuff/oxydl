//! Settings configuration dialog.

use egui::{Color32, RichText, Window};
use crate::manager::storage::AppSettings;
use crate::util::file::get_app_data_dir;

pub struct SettingsDialogState {
    pub is_open: bool,
    pub settings: AppSettings,
}

impl SettingsDialogState {
    pub fn new(settings: AppSettings) -> Self {
        Self {
            is_open: false,
            settings,
        }
    }

    pub fn open(&mut self, current: AppSettings) {
        self.settings = current;
        self.is_open = true;
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
    Window::new("⚙ Oxidm Preferences & Settings")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(500.0)
        .show(ctx, |ui| {
            ui.add_space(4.0);

            // Default save folder
            ui.label(RichText::new("Default Download Folder:").strong());
            ui.horizontal(|ui| {
                let mut path_str = state.settings.default_save_dir.to_string_lossy().to_string();
                ui.add(egui::TextEdit::singleline(&mut path_str).desired_width(400.0));

                if ui.button("📁 Browse...").clicked() {
                    if let Some(folder) = rfd::FileDialog::new()
                        .set_directory(&state.settings.default_save_dir)
                        .pick_folder()
                    {
                        state.settings.default_save_dir = folder;
                    }
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Concurrency Settings
            ui.horizontal(|ui| {
                ui.label("Max Concurrent Active Downloads:");
                egui::ComboBox::from_id_salt("max_active_dl")
                    .selected_text(format!("{} Downloads", state.settings.max_concurrent_downloads))
                    .show_ui(ui, |ui| {
                        for count in 1..=10 {
                            ui.selectable_value(
                                &mut state.settings.max_concurrent_downloads,
                                count,
                                format!("{} Downloads", count),
                            );
                        }
                    });
            });

            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.label("Default Connections (Parts) per Download:");
                egui::ComboBox::from_id_salt("default_parts")
                    .selected_text(format!("{} Parts", state.settings.default_connections))
                    .show_ui(ui, |ui| {
                        for &count in &[1, 2, 4, 8, 16, 24, 32] {
                            ui.selectable_value(
                                &mut state.settings.default_connections,
                                count,
                                format!("{} Parts", count),
                            );
                        }
                    });
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Clipboard Integration
            ui.checkbox(
                &mut state.settings.monitor_clipboard,
                "Monitor system clipboard for copied download links (IDM-style)",
            );

            ui.add_space(4.0);

            // Dark Mode
            ui.checkbox(&mut state.settings.dark_mode, "Enable Dark Theme");

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            // Storage mode information badge
            let data_dir = get_app_data_dir();
            let is_portable = data_dir.ends_with("data");
            ui.horizontal(|ui| {
                ui.label(RichText::new("Data Mode:").strong());
                if is_portable {
                    ui.label(
                        RichText::new("Portable Mode (Local directory)")
                            .color(Color32::from_rgb(46, 204, 113))
                            .strong(),
                    );
                } else {
                    ui.label(
                        RichText::new("Standard System Mode")
                            .color(Color32::from_rgb(52, 152, 219)),
                    );
                }
            });
            ui.label(
                RichText::new(format!("Location: {}", data_dir.display()))
                    .size(11.0)
                    .color(Color32::from_rgb(140, 145, 160)),
            );

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);

            // Action Buttons
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
