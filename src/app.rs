//! Main eframe application managing UI layout, state synchronization, and event dispatching.

use std::time::Duration;
use egui::{Color32, RichText, TopBottomPanel, CentralPanel, SidePanel};
use uuid::Uuid;

use crate::manager::queue::QueueManager;
use crate::manager::task::{DownloadTask, TaskStatus};
use crate::ui::add_dialog::{render_add_dialog, AddDialogResult, AddDialogState};
use crate::ui::segment_bar::render_segment_visualizer;
use crate::ui::settings_dialog::{render_settings_dialog, SettingsDialogResult, SettingsDialogState};
use crate::ui::sidebar::{render_sidebar, FilterSelection};
use crate::ui::table::{render_table, TableAction};
use crate::ui::toolbar::{render_toolbar, ToolbarAction};
use crate::util::clipboard::ClipboardMonitor;
use crate::util::format::format_speed;

pub struct OxidmApp {
    queue: QueueManager,
    clipboard_monitor: ClipboardMonitor,
    selected_task_id: Option<Uuid>,
    current_filter: FilterSelection,
    add_dialog: AddDialogState,
    settings_dialog: SettingsDialogState,
    clipboard_prompt_url: Option<String>,
    cached_tasks: Vec<DownloadTask>,
    current_speed_limit_kbps: Option<u64>,
}

impl OxidmApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let queue = QueueManager::new();
        let settings = {
            // Synchronously inspect initial settings
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    queue.settings.lock().await.clone()
                })
            })
        };

        let add_dialog = AddDialogState::new(
            settings.default_save_dir.clone(),
            settings.default_connections,
        );
        let settings_dialog = SettingsDialogState::new(settings.clone());

        // Start local HTTP API server for browser extension integration (Chrome / Firefox / Edge)
        let api_server = std::sync::Arc::new(crate::api::ApiServer::new(
            queue.clone(),
            crate::api::DEFAULT_API_PORT,
        ));
        api_server.start();

        Self {
            queue,
            clipboard_monitor: ClipboardMonitor::new(),
            selected_task_id: None,
            current_filter: FilterSelection::All,
            add_dialog,
            settings_dialog,
            clipboard_prompt_url: None,
            cached_tasks: Vec::new(),
            current_speed_limit_kbps: settings.speed_limit_kbps,
        }
    }
}

impl eframe::App for OxidmApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Try pulling latest snapshot of tasks without blocking GUI thread
        if let Ok(tasks_guard) = self.queue.tasks.try_lock() {
            self.cached_tasks = tasks_guard.clone();
        }

        // Check if any downloads are active to determine repaint rate
        let has_active_downloads = self
            .cached_tasks
            .iter()
            .any(|t| t.status == TaskStatus::Downloading);

        // Check clipboard if enabled in settings
        let monitor_enabled = if let Ok(s) = self.queue.settings.try_lock() {
            s.monitor_clipboard
        } else {
            true
        };

        if monitor_enabled && !self.add_dialog.is_open && self.clipboard_prompt_url.is_none() {
            if let Some(new_url) = self.clipboard_monitor.poll_new_url() {
                // Ensure this URL is not already in the download list
                let already_exists = self.cached_tasks.iter().any(|t| t.url == new_url);
                if !already_exists {
                    self.clipboard_prompt_url = Some(new_url);
                }
            }
        }

        // Clipboard URL Banner Notification
        if let Some(ref prompt_url) = self.clipboard_prompt_url.clone() {
            TopBottomPanel::top("clipboard_banner").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("📋 Download Link Copied:")
                            .strong()
                            .color(Color32::from_rgb(52, 152, 219)),
                    );

                    let short_url = if prompt_url.len() > 60 {
                        format!("{}...", &prompt_url[..57])
                    } else {
                        prompt_url.clone()
                    };
                    ui.label(short_url);

                    let add_btn = egui::Button::new(
                        RichText::new("⚡ Download Now")
                            .color(Color32::WHITE)
                            .strong(),
                    )
                    .fill(Color32::from_rgb(46, 204, 113));

                    if ui.add(add_btn).clicked() {
                        self.add_dialog.open_with_url(prompt_url.clone());
                        self.clipboard_prompt_url = None;
                    }

                    if ui.button("Dismiss").clicked() {
                        self.clipboard_prompt_url = None;
                    }
                });
            });
        }

        // Top Toolbar
        let selected_task = self
            .selected_task_id
            .and_then(|id| self.cached_tasks.iter().find(|t| t.id == id));
        let selected_status = selected_task.map(|t| &t.status);

        TopBottomPanel::top("main_toolbar").show(ctx, |ui| {
            ui.add_space(4.0);
            let action = render_toolbar(ui, selected_status, self.current_speed_limit_kbps);
            ui.add_space(4.0);

            match action {
                ToolbarAction::None => {}
                ToolbarAction::OpenAddDialog => {
                    self.add_dialog.is_open = true;
                    // Auto-paste if valid clipboard URL
                    if let Some(url) = self.clipboard_monitor.poll_new_url() {
                        self.add_dialog.open_with_url(url);
                    }
                }
                ToolbarAction::ResumeSelected => {
                    if let Some(id) = self.selected_task_id {
                        let q = self.queue.clone();
                        tokio::spawn(async move {
                            q.start_task(id).await;
                        });
                    }
                }
                ToolbarAction::PauseSelected => {
                    if let Some(id) = self.selected_task_id {
                        let q = self.queue.clone();
                        tokio::spawn(async move {
                            q.pause_task(id).await;
                        });
                    }
                }
                ToolbarAction::ResumeAll => {
                    let q = self.queue.clone();
                    tokio::spawn(async move {
                        q.resume_all().await;
                    });
                }
                ToolbarAction::PauseAll => {
                    let q = self.queue.clone();
                    tokio::spawn(async move {
                        q.pause_all().await;
                    });
                }
                ToolbarAction::DeleteSelected => {
                    if let Some(id) = self.selected_task_id {
                        let q = self.queue.clone();
                        tokio::spawn(async move {
                            q.delete_task(id, false).await;
                        });
                        self.selected_task_id = None;
                    }
                }
                ToolbarAction::OpenSettings => {
                    if let Ok(settings) = self.queue.settings.try_lock() {
                        self.settings_dialog.open(settings.clone());
                    }
                }
                ToolbarAction::SetSpeedLimit(limit) => {
                    self.current_speed_limit_kbps = limit;
                    let q = self.queue.clone();
                    tokio::spawn(async move {
                        let mut s = q.settings.lock().await;
                        s.speed_limit_kbps = limit;
                        let s_clone = s.clone();
                        drop(s);
                        q.update_settings(s_clone).await;
                    });
                }
            }
        });

        // Bottom Detail Panel (Segment Bar or Status Footer)
        TopBottomPanel::bottom("bottom_detail_panel")
            .resizable(true)
            .min_height(90.0)
            .default_height(140.0)
            .show(ctx, |ui| {
                if let Some(task) = selected_task {
                    render_segment_visualizer(ui, task);
                } else {
                    // Global Footer
                    ui.vertical(|ui| {
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            let total_speed: f64 = self
                                .cached_tasks
                                .iter()
                                .filter(|t| t.status == TaskStatus::Downloading)
                                .map(|t| t.speed_bps)
                                .sum();

                            let active_count = self
                                .cached_tasks
                                .iter()
                                .filter(|t| t.status == TaskStatus::Downloading)
                                .count();

                            ui.label(RichText::new("Oxidm Status:").strong());
                            ui.label(format!("Active Downloads: {}", active_count));
                            ui.separator();
                            ui.label(format!("Total Speed: {}", format_speed(total_speed)));
                            ui.separator();
                            ui.label(format!("Total Tasks: {}", self.cached_tasks.len()));

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(
                                    RichText::new("Pure Rust • Zero Bloat • High Performance")
                                        .size(11.0)
                                        .color(Color32::from_rgb(140, 145, 160)),
                                );
                            });
                        });
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("Tip: Select any download above to inspect live multi-part segment streams.")
                                .size(11.0)
                                .color(Color32::from_rgb(120, 125, 140)),
                        );
                    });
                }
            });

        // Left Navigation Sidebar
        SidePanel::left("left_sidebar")
            .resizable(false)
            .default_width(190.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                render_sidebar(ui, &mut self.current_filter, &self.cached_tasks);
            });

        // Central Panel: Downloads Table
        CentralPanel::default().show(ctx, |ui| {
            // Filter tasks based on current selection
            let filtered_tasks: Vec<DownloadTask> = self
                .cached_tasks
                .iter()
                .filter(|task| match self.current_filter {
                    FilterSelection::All => true,
                    FilterSelection::Downloading => task.status == TaskStatus::Downloading,
                    FilterSelection::Paused => {
                        matches!(task.status, TaskStatus::Paused | TaskStatus::Queued)
                    }
                    FilterSelection::Completed => task.status == TaskStatus::Completed,
                    FilterSelection::Category(cat) => task.category == cat,
                })
                .cloned()
                .collect();

            let table_action = render_table(ui, &filtered_tasks, self.selected_task_id);

            match table_action {
                TableAction::None => {}
                TableAction::Select(id) => {
                    self.selected_task_id = Some(id);
                }
                TableAction::Resume(id) => {
                    let q = self.queue.clone();
                    tokio::spawn(async move {
                        q.start_task(id).await;
                    });
                }
                TableAction::Pause(id) => {
                    let q = self.queue.clone();
                    tokio::spawn(async move {
                        q.pause_task(id).await;
                    });
                }
                TableAction::Redownload(id) => {
                    let q = self.queue.clone();
                    tokio::spawn(async move {
                        q.redownload_task(id).await;
                    });
                }
                TableAction::Delete(id, delete_from_disk) => {
                    let q = self.queue.clone();
                    tokio::spawn(async move {
                        q.delete_task(id, delete_from_disk).await;
                    });
                    if self.selected_task_id == Some(id) {
                        self.selected_task_id = None;
                    }
                }
                TableAction::OpenFile(path) => {
                    let _ = open::that(path);
                }
                TableAction::OpenFolder(path) => {
                    let _ = open::that(path);
                }
                TableAction::CopyLink(url) => {
                    if let Ok(mut cb) = arboard::Clipboard::new() {
                        let _ = cb.set_text(url);
                    }
                }
            }
        });

        // Dialog Modals
        let add_res = render_add_dialog(ctx, &mut self.add_dialog);
        match add_res {
            AddDialogResult::None | AddDialogResult::Cancel => {}
            AddDialogResult::DownloadNow {
                url,
                file_name,
                save_dir,
                num_connections,
            } => {
                let q = self.queue.clone();
                tokio::spawn(async move {
                    let _id = q.add_task(url, file_name, save_dir, num_connections, true).await;
                });
            }
            AddDialogResult::QueueOnly {
                url,
                file_name,
                save_dir,
                num_connections,
            } => {
                let q = self.queue.clone();
                tokio::spawn(async move {
                    q.add_task(url, file_name, save_dir, num_connections, false).await;
                });
            }
        }

        let settings_res = render_settings_dialog(ctx, &mut self.settings_dialog);
        if let SettingsDialogResult::Save(new_settings) = settings_res {
            let q = self.queue.clone();
            tokio::spawn(async move {
                q.update_settings(new_settings).await;
            });
        }

        // Schedule repaint: fast if downloading, idle otherwise
        if has_active_downloads {
            ctx.request_repaint_after(Duration::from_millis(100));
        } else {
            ctx.request_repaint_after(Duration::from_millis(300));
        }
    }
}
