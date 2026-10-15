//! "Download & Speed Scheduler" modal dialog.

use egui::{Color32, RichText, Ui, Window};

use crate::manager::scheduler::{QueueSchedule, ScheduleAction, SchedulerConfig, SpeedSchedule};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerTab {
    QueueTimers,
    SpeedSchedule,
}

pub struct SchedulerDialogState {
    pub is_open: bool,
    pub active_tab: SchedulerTab,
    pub config: SchedulerConfig,
    pub saved_notice: bool,
}

impl SchedulerDialogState {
    pub fn new(initial_config: SchedulerConfig) -> Self {
        Self {
            is_open: false,
            active_tab: SchedulerTab::QueueTimers,
            config: initial_config,
            saved_notice: false,
        }
    }

    pub fn open(&mut self, config: SchedulerConfig) {
        self.config = config;
        self.saved_notice = false;
        self.is_open = true;
    }
}

pub enum SchedulerDialogResult {
    None,
    Save(SchedulerConfig),
}

pub fn render_scheduler_dialog(
    ctx: &egui::Context,
    state: &mut SchedulerDialogState,
) -> SchedulerDialogResult {
    let mut result = SchedulerDialogResult::None;

    if !state.is_open {
        return result;
    }

    let mut open = state.is_open;
    Window::new("⏰ Download & Speed Scheduler")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(520.0)
        .show(ctx, |ui| {
            ui.add_space(4.0);

            // Tab navigation bar
            ui.horizontal(|ui| {
                if ui.selectable_label(state.active_tab == SchedulerTab::QueueTimers, "📅 Queue Timers").clicked() {
                    state.active_tab = SchedulerTab::QueueTimers;
                }
                if ui.selectable_label(state.active_tab == SchedulerTab::SpeedSchedule, "⚡ Peak-Hours Limiter").clicked() {
                    state.active_tab = SchedulerTab::SpeedSchedule;
                }
            });

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(6.0);

            match state.active_tab {
                SchedulerTab::QueueTimers => {
                    render_queue_tab(ui, &mut state.config.queue_schedule);
                }
                SchedulerTab::SpeedSchedule => {
                    render_speed_tab(ui, &mut state.config.speed_schedule);
                }
            }

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(6.0);

            // Footer action buttons
            ui.horizontal(|ui| {
                if ui.button(RichText::new("💾 Save Schedule").strong().color(Color32::WHITE)).clicked() {
                    result = SchedulerDialogResult::Save(state.config.clone());
                    state.is_open = false;
                }

                if ui.button("Cancel").clicked() {
                    state.is_open = false;
                }

                if state.config.queue_schedule.enabled || state.config.speed_schedule.enabled {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new("● Scheduler Active")
                                .color(Color32::from_rgb(46, 204, 113))
                                .strong(),
                        );
                    });
                }
            });
        });

    state.is_open = open;
    result
}

fn render_queue_tab(ui: &mut Ui, schedule: &mut QueueSchedule) {
    ui.checkbox(&mut schedule.enabled, RichText::new("Enable Automated Queue Scheduling").strong());
    ui.add_space(6.0);

    ui.group(|ui| {
        ui.label(RichText::new("Automated Start & Stop Times (24-hour format):").strong());
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("▶ Start Queue At:");
            ui.add(egui::TextEdit::singleline(&mut schedule.start_time).desired_width(70.0).hint_text("02:00"));
            ui.label("(HH:MM)");
        });

        ui.horizontal(|ui| {
            ui.label("⏸ Stop Queue At:");
            ui.add(egui::TextEdit::singleline(&mut schedule.stop_time).desired_width(70.0).hint_text("07:00"));
            ui.label("(HH:MM)");
        });
    });

    ui.add_space(6.0);

    // Days of week selector
    ui.group(|ui| {
        ui.label(RichText::new("Active Days of the Week:").strong());
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            let days_labels = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
            for (idx, label) in days_labels.iter().enumerate() {
                let day_val = idx as u8;
                let mut checked = schedule.days_of_week.contains(&day_val);
                if ui.checkbox(&mut checked, *label).changed() {
                    if checked {
                        if !schedule.days_of_week.contains(&day_val) {
                            schedule.days_of_week.push(day_val);
                            schedule.days_of_week.sort();
                        }
                    } else {
                        schedule.days_of_week.retain(|&d| d != day_val);
                    }
                }
            }
        });
    });

    ui.add_space(6.0);

    // Action on completion
    ui.group(|ui| {
        ui.label(RichText::new("When All Scheduled Downloads Complete:").strong());
        ui.add_space(4.0);

        let actions = [
            ScheduleAction::None,
            ScheduleAction::PauseQueue,
            ScheduleAction::SleepComputer,
            ScheduleAction::ShutdownComputer,
        ];

        for action in actions {
            ui.radio_value(&mut schedule.action_on_complete, action, action.display_name());
        }
    });
}

fn render_speed_tab(ui: &mut Ui, schedule: &mut SpeedSchedule) {
    ui.checkbox(&mut schedule.enabled, RichText::new("Enable Peak-Hours Bandwidth Limiter").strong());
    ui.add_space(6.0);

    ui.group(|ui| {
        ui.label(RichText::new("Peak Hours Time Window:").strong());
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("From:");
            ui.add(egui::TextEdit::singleline(&mut schedule.peak_start).desired_width(70.0).hint_text("09:00"));
            ui.label("To:");
            ui.add(egui::TextEdit::singleline(&mut schedule.peak_end).desired_width(70.0).hint_text("18:00"));
        });
    });

    ui.add_space(6.0);

    ui.group(|ui| {
        ui.label(RichText::new("Speed Limits (KB/s):").strong());
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("⚡ Peak Limit:");
            ui.add(egui::DragValue::new(&mut schedule.peak_limit_kbps).speed(50).range(50..=100_000));
            ui.label("KB/s");
        });

        ui.horizontal(|ui| {
            let mut off_peak_unlimited = schedule.off_peak_limit_kbps.is_none();
            if ui.checkbox(&mut off_peak_unlimited, "Off-Peak Unlimited Speed").changed() {
                if off_peak_unlimited {
                    schedule.off_peak_limit_kbps = None;
                } else {
                    schedule.off_peak_limit_kbps = Some(2000);
                }
            }

            if let Some(ref mut off_limit) = schedule.off_peak_limit_kbps {
                ui.label("Limit:");
                ui.add(egui::DragValue::new(off_limit).speed(50).range(50..=100_000));
                ui.label("KB/s");
            }
        });
    });

    ui.add_space(6.0);

    ui.group(|ui| {
        ui.label(RichText::new("Active Peak Days:").strong());
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            let days_labels = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
            for (idx, label) in days_labels.iter().enumerate() {
                let day_val = idx as u8;
                let mut checked = schedule.peak_days.contains(&day_val);
                if ui.checkbox(&mut checked, *label).changed() {
                    if checked {
                        if !schedule.peak_days.contains(&day_val) {
                            schedule.peak_days.push(day_val);
                            schedule.peak_days.sort();
                        }
                    } else {
                        schedule.peak_days.retain(|&d| d != day_val);
                    }
                }
            }
        });
    });
}
