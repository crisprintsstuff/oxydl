//! Top action toolbar providing download controls and shortcuts.

use egui::{Color32, RichText, Ui};

use crate::manager::task::TaskStatus;

#[derive(Debug, Clone, PartialEq)]
pub enum ToolbarAction {
    None,
    OpenAddDialog,
    ResumeSelected,
    PauseSelected,
    PauseAll,
    ResumeAll,
    DeleteSelected,
    OpenSettings,
    OpenScheduler,
    OpenConverter,
    SetSpeedLimit(Option<u64>),
}

pub fn render_toolbar(
    ui: &mut Ui,
    selected_task_status: Option<&TaskStatus>,
    current_speed_limit_kbps: Option<u64>,
) -> ToolbarAction {
    let mut action = ToolbarAction::None;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        // Primary "+ Add URL" Button
        let add_btn = egui::Button::new(
            RichText::new("➕ Add URL")
                .strong()
                .color(Color32::WHITE),
        )
        .fill(Color32::from_rgb(41, 128, 185)); // Accent Blue

        if ui.add(add_btn).clicked() {
            action = ToolbarAction::OpenAddDialog;
        }

        ui.separator();

        // Resume Button
        let can_resume = selected_task_status.map(|s| s.can_resume()).unwrap_or(false);
        let resume_btn = egui::Button::new("▶ Resume");
        if ui.add_enabled(can_resume, resume_btn).clicked() {
            action = ToolbarAction::ResumeSelected;
        }

        // Pause Button
        let can_pause = selected_task_status.map(|s| s.is_active()).unwrap_or(false);
        let pause_btn = egui::Button::new("⏸ Pause");
        if ui.add_enabled(can_pause, pause_btn).clicked() {
            action = ToolbarAction::PauseSelected;
        }

        // Resume All / Pause All
        if ui.button("▶▶ Resume All").clicked() {
            action = ToolbarAction::ResumeAll;
        }

        if ui.button("⏸⏸ Pause All").clicked() {
            action = ToolbarAction::PauseAll;
        }

        ui.separator();

        // Delete Button
        let has_selection = selected_task_status.is_some();
        let del_btn = egui::Button::new(
            RichText::new("🗑 Delete").color(if has_selection {
                Color32::from_rgb(231, 76, 60)
            } else {
                Color32::GRAY
            }),
        );
        if ui.add_enabled(has_selection, del_btn).clicked() {
            action = ToolbarAction::DeleteSelected;
        }

        ui.separator();

        // Speed Limiter Dropdown
        ui.label("⚡ Speed Limit:");
        let current_text = match current_speed_limit_kbps {
            None => "Unlimited".to_string(),
            Some(kb) if kb >= 1024 => format!("{:.1} MB/s", (kb as f64) / 1024.0),
            Some(kb) => format!("{} KB/s", kb),
        };

        egui::ComboBox::from_id_salt("speed_limit_combo")
            .selected_text(current_text)
            .show_ui(ui, |ui| {
                if ui.selectable_label(current_speed_limit_kbps.is_none(), "Unlimited").clicked() {
                    action = ToolbarAction::SetSpeedLimit(None);
                }
                if ui.selectable_label(current_speed_limit_kbps == Some(500), "500 KB/s").clicked() {
                    action = ToolbarAction::SetSpeedLimit(Some(500));
                }
                if ui.selectable_label(current_speed_limit_kbps == Some(1024), "1 MB/s").clicked() {
                    action = ToolbarAction::SetSpeedLimit(Some(1024));
                }
                if ui.selectable_label(current_speed_limit_kbps == Some(2048), "2 MB/s").clicked() {
                    action = ToolbarAction::SetSpeedLimit(Some(2048));
                }
                if ui.selectable_label(current_speed_limit_kbps == Some(5120), "5 MB/s").clicked() {
                    action = ToolbarAction::SetSpeedLimit(Some(5120));
                }
                if ui.selectable_label(current_speed_limit_kbps == Some(10240), "10 MB/s").clicked() {
                    action = ToolbarAction::SetSpeedLimit(Some(10240));
                }
            });

        // Right side: Settings, Scheduler, Converter
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("⚙ Settings").clicked() {
                action = ToolbarAction::OpenSettings;
            }
            if ui.button("⏰ Scheduler").clicked() {
                action = ToolbarAction::OpenScheduler;
            }
            if ui.button("🎬 Converter").clicked() {
                action = ToolbarAction::OpenConverter;
            }
        });
    });

    action
}
