//! IDM-style segment visualizer rendering multi-connection chunk progress.

use egui::{Color32, Pos2, Rect, Stroke, Ui, Vec2};
use crate::engine::state::{ChunkState, ChunkStatus};
use crate::manager::task::DownloadTask;
use crate::util::format::{format_bytes, format_eta, format_speed};

pub fn render_segment_visualizer(ui: &mut Ui, task: &DownloadTask) {
    ui.group(|ui| {
        ui.vertical(|ui| {
            // Header with stats
            ui.horizontal(|ui| {
                ui.heading(format!("{} {}", task.category.icon(), task.file_name));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(total) = task.total_bytes {
                        ui.label(format!(
                            "{:.1}% of {}",
                            task.fraction() * 100.0,
                            format_bytes(total)
                        ));
                    } else {
                        ui.label(format!("Downloaded: {}", format_bytes(task.downloaded_bytes)));
                    }
                });
            });

            ui.separator();

            // Status and metrics strip
            ui.horizontal(|ui| {
                ui.label(format!("Status: {}", task.status.display_name()));
                ui.separator();
                ui.label(format!("Speed: {}", format_speed(task.speed_bps)));
                ui.separator();
                ui.label(format!("ETA: {}", format_eta(task.eta_seconds)));
                ui.separator();
                ui.label(format!("Connections: {}", task.num_connections));
            });

            ui.add_space(8.0);

            // Title for chunk bar
            ui.label(
                egui::RichText::new("Connection Streams (Segments):")
                    .strong()
                    .size(12.0),
            );

            // If we have chunks, draw the multi-segment bar
            if !task.chunks.is_empty() {
                render_chunk_canvas(ui, &task.chunks, task.total_bytes);
            } else {
                // Fallback single progress bar
                let frac = task.fraction();
                ui.add(egui::ProgressBar::new(frac).show_percentage());
            }

            ui.add_space(4.0);

            // Chunk details list (scrollable if many chunks)
            if !task.chunks.is_empty() {
                render_chunk_grid(ui, &task.chunks);
            }
        });
    });
}

fn render_chunk_canvas(ui: &mut Ui, chunks: &[ChunkState], total_bytes: Option<u64>) {
    let desired_size = Vec2::new(ui.available_width(), 26.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

    let painter = ui.painter_at(rect);
    let bg_color = Color32::from_rgb(30, 32, 38);
    let border_color = Color32::from_rgb(60, 65, 78);

    // Draw background container
    painter.rect(
        rect,
        3.0,
        bg_color,
        Stroke::new(1.0_f32, border_color),
    );

    let total = match total_bytes {
        Some(t) if t > 0 => t as f64,
        _ => chunks.iter().map(|c| c.total_bytes()).sum::<u64>().max(1) as f64,
    };

    let inner_rect = rect.shrink(1.5);
    let total_width = inner_rect.width();

    for chunk in chunks {
        let chunk_start_frac = (chunk.start as f64 / total).clamp(0.0, 1.0) as f32;
        let chunk_len_frac = (chunk.total_bytes() as f64 / total).clamp(0.0, 1.0) as f32;

        let chunk_x = inner_rect.min.x + chunk_start_frac * total_width;
        let chunk_w = (chunk_len_frac * total_width).max(1.0);

        let chunk_rect = Rect::from_min_size(
            Pos2::new(chunk_x, inner_rect.min.y),
            Vec2::new(chunk_w, inner_rect.height()),
        );

        // Chunk pending background
        let segment_bg = Color32::from_rgb(45, 48, 58);
        painter.rect_filled(chunk_rect, 0.0, segment_bg);

        // Downloaded portion
        let downloaded_frac = chunk.fraction();
        let dl_w = chunk_w * downloaded_frac;

        let dl_color = match chunk.status {
            ChunkStatus::Completed => Color32::from_rgb(46, 204, 113), // Emerald Green
            ChunkStatus::Downloading => Color32::from_rgb(52, 152, 219), // Vibrant Blue
            ChunkStatus::Failed => Color32::from_rgb(231, 76, 60),     // Red
            ChunkStatus::Pending => Color32::from_rgb(100, 105, 120),  // Muted gray
        };

        if dl_w > 0.5 {
            let dl_rect = Rect::from_min_size(
                chunk_rect.min,
                Vec2::new(dl_w, chunk_rect.height()),
            );
            painter.rect_filled(dl_rect, 0.0, dl_color);
        }

        // Divider line between segments
        painter.line_segment(
            [
                Pos2::new(chunk_x + chunk_w, inner_rect.min.y),
                Pos2::new(chunk_x + chunk_w, inner_rect.max.y),
            ],
            Stroke::new(1.0_f32, Color32::from_rgb(20, 22, 26)),
        );
    }

    if response.hovered() {
        response.on_hover_text("IDM-style multi-part segment visualizer");
    }
}

fn render_chunk_grid(ui: &mut Ui, chunks: &[ChunkState]) {
    egui::ScrollArea::horizontal().show(ui, |ui| {
        ui.horizontal(|ui| {
            for chunk in chunks {
                let badge_color = match chunk.status {
                    ChunkStatus::Completed => Color32::from_rgb(46, 204, 113),
                    ChunkStatus::Downloading => Color32::from_rgb(52, 152, 219),
                    ChunkStatus::Failed => Color32::from_rgb(231, 76, 60),
                    ChunkStatus::Pending => Color32::GRAY,
                };

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(8.0, 8.0), egui::Sense::hover());
                        ui.painter().circle_filled(rect.center(), 4.0, badge_color);
                        ui.label(
                            egui::RichText::new(format!("#{}", chunk.index + 1))
                                .size(11.0)
                                .strong(),
                        );
                    });
                    ui.label(
                        egui::RichText::new(format!("{:.0}%", chunk.fraction() * 100.0))
                            .size(10.0),
                    );
                    ui.label(
                        egui::RichText::new(format_bytes(chunk.downloaded))
                            .size(10.0)
                            .color(Color32::from_rgb(170, 175, 190)),
                    );
                });
                ui.add_space(8.0);
            }
        });
    });
}
