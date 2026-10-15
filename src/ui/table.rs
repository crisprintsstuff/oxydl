//! Main downloads list table with progress bars, sorting, and context menu.

use std::path::PathBuf;
use egui::{Color32, RichText, Sense, Ui, Vec2};
use uuid::Uuid;

use crate::manager::task::{DownloadTask, TaskStatus};
use crate::util::format::{format_bytes, format_eta, format_speed};

#[derive(Debug, Clone, PartialEq)]
pub enum TableAction {
    None,
    Select(Uuid),
    Resume(Uuid),
    Pause(Uuid),
    Redownload(Uuid),
    Delete(Uuid, bool), // (task_id, delete_from_disk)
    OpenFile(PathBuf),
    OpenFolder(PathBuf),
    CopyLink(String),
}

pub fn render_table(
    ui: &mut Ui,
    tasks: &[DownloadTask],
    selected_id: Option<Uuid>,
) -> TableAction {
    let mut action = TableAction::None;

    if tasks.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(
                RichText::new("No downloads in this category.")
                    .size(16.0)
                    .color(Color32::from_rgb(140, 145, 160)),
            );
            ui.label("Click '➕ Add URL' to start downloading with maximum speed.");
        });
        return action;
    }

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("downloads_grid")
                .striped(true)
                .min_col_width(80.0)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    // Table Header
                    ui.label(RichText::new("File Name").strong());
                    ui.label(RichText::new("Size").strong());
                    ui.label(RichText::new("Progress").strong());
                    ui.label(RichText::new("Status").strong());
                    ui.label(RichText::new("Speed").strong());
                    ui.label(RichText::new("ETA").strong());
                    ui.label(RichText::new("Added").strong());
                    ui.end_row();

                    // Table Rows
                    for task in tasks {
                        let is_selected = selected_id == Some(task.id);

                        // Column 1: File Name (with icon and selection highlight)
                        let file_label = format!("{} {}", task.category.icon(), task.file_name);
                        let name_text = if is_selected {
                            RichText::new(&file_label)
                                .strong()
                                .color(Color32::from_rgb(52, 152, 219))
                        } else {
                            RichText::new(&file_label)
                        };

                        let name_resp = ui.add(egui::Label::new(name_text).sense(Sense::click()));
                        if name_resp.clicked() {
                            action = TableAction::Select(task.id);
                        }
                        if name_resp.double_clicked() {
                            if task.status == TaskStatus::Completed {
                                action = TableAction::OpenFile(task.save_path.clone());
                            } else if task.status == TaskStatus::Downloading {
                                action = TableAction::Pause(task.id);
                            } else if task.status.can_resume() {
                                action = TableAction::Resume(task.id);
                            }
                        }

                        // Context menu on right-click
                        name_resp.context_menu(|ui| {
                            if task.status == TaskStatus::Completed {
                                if ui.button("📂 Open File").clicked() {
                                    action = TableAction::OpenFile(task.save_path.clone());
                                    ui.close_menu();
                                }
                            }
                            if ui.button("📁 Open Containing Folder").clicked() {
                                if let Some(parent) = task.save_path.parent() {
                                    action = TableAction::OpenFolder(parent.to_path_buf());
                                }
                                ui.close_menu();
                            }
                            ui.separator();
                            if task.status.can_resume() {
                                if ui.button("▶ Resume").clicked() {
                                    action = TableAction::Resume(task.id);
                                    ui.close_menu();
                                }
                            }
                            if task.status == TaskStatus::Downloading {
                                if ui.button("⏸ Pause").clicked() {
                                    action = TableAction::Pause(task.id);
                                    ui.close_menu();
                                }
                            }
                            if ui.button("🔄 Redownload").clicked() {
                                action = TableAction::Redownload(task.id);
                                ui.close_menu();
                            }
                            if ui.button("📋 Copy URL").clicked() {
                                action = TableAction::CopyLink(task.url.clone());
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui.button("🗑 Remove from List").clicked() {
                                action = TableAction::Delete(task.id, false);
                                ui.close_menu();
                            }
                            if ui.button("💥 Delete from Disk").clicked() {
                                action = TableAction::Delete(task.id, true);
                                ui.close_menu();
                            }
                        });

                        // Column 2: File Size
                        let size_text = match task.total_bytes {
                            Some(tot) => format_bytes(tot),
                            None => format!("{} (Unknown)", format_bytes(task.downloaded_bytes)),
                        };
                        ui.label(size_text);

                        // Column 3: Progress Bar
                        let frac = task.fraction();
                        let pbar = egui::ProgressBar::new(frac)
                            .desired_width(120.0)
                            .text(format!("{:.1}%", frac * 100.0));
                        ui.add(pbar);

                        // Column 4: Status Badge
                        render_status_badge(ui, &task.status);

                        // Column 5: Speed
                        if task.status == TaskStatus::Downloading {
                            ui.label(
                                RichText::new(format_speed(task.speed_bps))
                                    .color(Color32::from_rgb(46, 204, 113))
                                    .strong(),
                            );
                        } else {
                            ui.label("--");
                        }

                        // Column 6: ETA
                        if task.status == TaskStatus::Downloading {
                            ui.label(format_eta(task.eta_seconds));
                        } else {
                            ui.label("--");
                        }

                        // Column 7: Date Added
                        ui.label(task.created_at.format("%b %d, %H:%M").to_string());

                        ui.end_row();
                    }
                });
        });

    action
}

fn render_status_badge(ui: &mut Ui, status: &TaskStatus) {
    let (text, color) = match status {
        TaskStatus::Completed => ("Completed", Color32::from_rgb(46, 204, 113)),
        TaskStatus::Downloading => ("Downloading", Color32::from_rgb(52, 152, 219)),
        TaskStatus::Paused => ("Paused", Color32::from_rgb(243, 156, 18)),
        TaskStatus::Queued => ("Queued", Color32::from_rgb(155, 89, 182)),
        TaskStatus::Failed(_) => ("Error", Color32::from_rgb(231, 76, 60)),
    };

    let (rect, _) = ui.allocate_exact_size(Vec2::new(75.0, 18.0), Sense::hover());
    ui.painter().rect_filled(rect, 3.0, color.gamma_multiply(0.2));
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(11.0),
        color,
    );
}
