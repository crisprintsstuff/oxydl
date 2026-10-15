//! Left sidebar navigation for status and category filtering.

use egui::{Color32, RichText, Ui};
use crate::manager::task::{DownloadTask, TaskStatus};
use crate::util::file::TaskCategory;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterSelection {
    All,
    Downloading,
    Paused,
    Completed,
    Category(TaskCategory),
}

pub fn render_sidebar(
    ui: &mut Ui,
    current_filter: &mut FilterSelection,
    tasks: &[DownloadTask],
) {
    ui.vertical(|ui| {
        ui.heading("Categories");
        ui.add_space(4.0);

        // Status filters section
        let total_count = tasks.len();
        let downloading_count = tasks.iter().filter(|t| t.status == TaskStatus::Downloading).count();
        let paused_count = tasks.iter().filter(|t| matches!(t.status, TaskStatus::Paused | TaskStatus::Queued)).count();
        let completed_count = tasks.iter().filter(|t| t.status == TaskStatus::Completed).count();

        filter_item(ui, current_filter, FilterSelection::All, "📁 All Downloads", total_count);
        filter_item(ui, current_filter, FilterSelection::Downloading, "⚡ Downloading", downloading_count);
        filter_item(ui, current_filter, FilterSelection::Paused, "⏸ Paused / Queued", paused_count);
        filter_item(ui, current_filter, FilterSelection::Completed, "✅ Completed", completed_count);

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(4.0);
        ui.label(RichText::new("File Types").strong().size(12.0));
        ui.add_space(4.0);

        // Category filters
        let categories = [
            TaskCategory::Compressed,
            TaskCategory::Documents,
            TaskCategory::Music,
            TaskCategory::Videos,
            TaskCategory::Programs,
            TaskCategory::Other,
        ];

        for cat in categories {
            let count = tasks.iter().filter(|t| t.category == cat).count();
            let label = format!("{} {}", cat.icon(), cat.display_name());
            filter_item(ui, current_filter, FilterSelection::Category(cat), &label, count);
        }
    });
}

fn filter_item(
    ui: &mut Ui,
    current_filter: &mut FilterSelection,
    item_filter: FilterSelection,
    label: &str,
    count: usize,
) {
    let is_selected = *current_filter == item_filter;

    ui.horizontal(|ui| {
        let text = if is_selected {
            RichText::new(label).strong().color(Color32::from_rgb(52, 152, 219))
        } else {
            RichText::new(label)
        };

        if ui.selectable_label(is_selected, text).clicked() {
            *current_filter = item_filter;
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if count > 0 {
                let badge = RichText::new(count.to_string())
                    .size(11.0)
                    .color(if is_selected {
                        Color32::from_rgb(52, 152, 219)
                    } else {
                        Color32::from_rgb(140, 145, 160)
                    });
                ui.label(badge);
            }
        });
    });
}
