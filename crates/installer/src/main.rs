//! Native Windows Installer and Setup Wizard for OxyDL.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use eframe::egui::{self, Color32, ProgressBar, RichText};
use flate2::read::GzDecoder;

// Embedded compressed oxydl.exe binary payload
const OXYDL_PAYLOAD: &[u8] = include_bytes!("../payload/oxydl.exe.gz");
const APP_VERSION: &str = "0.1.0";
const APP_NAME: &str = "OxyDL";

#[derive(Debug, PartialEq, Eq)]
enum WizardStep {
    Welcome,
    SelectOptions,
    Installing,
    Installed,
    UninstallConfirm,
    Uninstalling,
    Uninstalled,
}

struct InstallerApp {
    step: WizardStep,
    install_dir: PathBuf,
    create_desktop_shortcut: bool,
    create_start_menu_shortcut: bool,
    register_uninstaller: bool,
    launch_after_install: bool,
    status_message: String,
    progress_frac: f32,
    error_message: Option<String>,
    is_uninstall_mode: bool,
}

impl InstallerApp {
    fn new(is_uninstall: bool) -> Self {
        let default_dir = get_default_install_dir();
        let step = if is_uninstall {
            WizardStep::UninstallConfirm
        } else {
            WizardStep::Welcome
        };

        Self {
            step,
            install_dir: default_dir,
            create_desktop_shortcut: true,
            create_start_menu_shortcut: true,
            register_uninstaller: true,
            launch_after_install: true,
            status_message: String::new(),
            progress_frac: 0.0,
            error_message: None,
            is_uninstall_mode: is_uninstall,
        }
    }

    fn perform_installation(&mut self) -> Result<(), String> {
        let target_dir = self.install_dir.clone();
        fs::create_dir_all(&target_dir)
            .map_err(|e| format!("Failed to create directory {}: {}", target_dir.display(), e))?;

        // 1. Decompress and extract oxydl.exe
        self.status_message = "Extracting oxydl.exe...".to_string();
        self.progress_frac = 0.3;

        let target_exe = target_dir.join("oxydl.exe");
        let mut decoder = GzDecoder::new(OXYDL_PAYLOAD);
        let mut out_file = File::create(&target_exe)
            .map_err(|e| format!("Failed to write oxydl.exe: {}", e))?;
        io::copy(&mut decoder, &mut out_file)
            .map_err(|e| format!("Failed to decompress payload: {}", e))?;

        // 2. Copy installer as uninstaller
        self.status_message = "Configuring uninstaller...".to_string();
        self.progress_frac = 0.5;

        let uninstaller_path = target_dir.join("uninstall.exe");
        if let Ok(current_exe) = std::env::current_exe() {
            let _ = fs::copy(&current_exe, &uninstaller_path);
        }

        // 3. Create Start Menu shortcut
        if self.create_start_menu_shortcut {
            self.status_message = "Creating Start Menu shortcut...".to_string();
            self.progress_frac = 0.7;

            if let Some(start_menu) = get_start_menu_programs_dir() {
                let _ = fs::create_dir_all(&start_menu);
                let lnk_path = start_menu.join("OxyDL.lnk");
                create_shortcut(&target_exe, &lnk_path, &target_dir);
            }
        }

        // 4. Create Desktop shortcut
        if self.create_desktop_shortcut {
            self.status_message = "Creating Desktop shortcut...".to_string();
            self.progress_frac = 0.85;

            if let Some(desktop) = get_desktop_dir() {
                let lnk_path = desktop.join("OxyDL.lnk");
                create_shortcut(&target_exe, &lnk_path, &target_dir);
            }
        }

        // 5. Register in Windows Registry (Add or Remove Programs)
        if self.register_uninstaller {
            self.status_message = "Registering with Windows Apps & Features...".to_string();
            self.progress_frac = 0.95;

            register_windows_uninstall(&target_dir, &target_exe, &uninstaller_path);
        }

        self.progress_frac = 1.0;
        self.status_message = "Installation completed successfully!".to_string();
        Ok(())
    }

    fn perform_uninstallation(&mut self) -> Result<(), String> {
        let install_dir = self.install_dir.clone();

        self.status_message = "Removing shortcuts...".to_string();
        self.progress_frac = 0.3;

        // Remove Start Menu shortcut
        if let Some(start_menu) = get_start_menu_programs_dir() {
            let lnk_path = start_menu.join("OxyDL.lnk");
            let _ = fs::remove_file(lnk_path);
        }

        // Remove Desktop shortcut
        if let Some(desktop) = get_desktop_dir() {
            let lnk_path = desktop.join("OxyDL.lnk");
            let _ = fs::remove_file(lnk_path);
        }

        // Remove Registry entries
        self.status_message = "Removing registry entries...".to_string();
        self.progress_frac = 0.6;
        unregister_windows_uninstall();

        // Remove files
        self.status_message = "Removing program files...".to_string();
        self.progress_frac = 0.9;
        let _ = fs::remove_file(install_dir.join("oxydl.exe"));

        self.progress_frac = 1.0;
        self.status_message = "OxyDL has been uninstalled.".to_string();
        Ok(())
    }
}

impl eframe::App for InstallerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let title = if self.is_uninstall_mode {
            "OxyDL Uninstaller"
        } else {
            "OxyDL Setup Wizard"
        };

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                // Header Banner
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🦀").size(32.0));
                    ui.vertical(|ui| {
                        ui.heading(RichText::new(title).size(20.0).strong());
                        ui.label(
                            RichText::new(format!("Version {}", APP_VERSION))
                                .size(11.0)
                                .color(Color32::from_rgb(140, 145, 160)),
                        );
                    });
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(12.0);

                // Body content based on WizardStep
                match self.step {
                    WizardStep::Welcome => {
                        ui.label(
                            RichText::new("Welcome to the OxyDL Setup Wizard")
                                .size(16.0)
                                .strong(),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            "This wizard will install OxyDL - High Speed Download Manager on your computer."
                        );
                        ui.add_space(6.0);
                        ui.label(
                            "OxyDL is a bloat-free, pure Rust alternative to Internet Download Manager (IDM), featuring multi-connection acceleration, IDM-style segment stream visualization, and automated clipboard link detection."
                        );
                        ui.add_space(16.0);
                        ui.label(
                            RichText::new("License: Open Source (MIT / Apache-2.0). 100% Free Forever.")
                                .color(Color32::from_rgb(46, 204, 113))
                                .strong(),
                        );

                        ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                            ui.horizontal(|ui| {
                                if ui.button(RichText::new("Next >").strong()).clicked() {
                                    self.step = WizardStep::SelectOptions;
                                }
                                if ui.button("Cancel").clicked() {
                                    std::process::exit(0);
                                }
                            });
                        });
                    }

                    WizardStep::SelectOptions => {
                        ui.label(RichText::new("Destination Folder:").strong());
                        ui.horizontal(|ui| {
                            let mut path_str = self.install_dir.to_string_lossy().to_string();
                            ui.add(egui::TextEdit::singleline(&mut path_str).desired_width(400.0));
                            if ui.button("📁 Browse...").clicked() {
                                if let Some(folder) = rfd::FileDialog::new()
                                    .set_directory(&self.install_dir)
                                    .pick_folder()
                                {
                                    self.install_dir = folder;
                                }
                            }
                        });

                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(8.0);

                        ui.label(RichText::new("Installation Options:").strong());
                        ui.checkbox(
                            &mut self.create_desktop_shortcut,
                            "Create a shortcut on the Desktop",
                        );
                        ui.checkbox(
                            &mut self.create_start_menu_shortcut,
                            "Add shortcut to Start Menu",
                        );
                        ui.checkbox(
                            &mut self.register_uninstaller,
                            "Register in Windows Apps & Features (Add or Remove Programs)",
                        );
                        ui.checkbox(
                            &mut self.launch_after_install,
                            "Launch OxyDL after installation finishes",
                        );

                        ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                            ui.horizontal(|ui| {
                                let install_btn = egui::Button::new(
                                    RichText::new("🚀 Install Now")
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(46, 204, 113));

                                if ui.add(install_btn).clicked() {
                                    self.step = WizardStep::Installing;
                                }
                                if ui.button("< Back").clicked() {
                                    self.step = WizardStep::Welcome;
                                }
                                if ui.button("Cancel").clicked() {
                                    std::process::exit(0);
                                }
                            });
                        });
                    }

                    WizardStep::Installing => {
                        ui.label(RichText::new("Installing OxyDL...").size(15.0).strong());
                        ui.add_space(12.0);

                        ui.add(ProgressBar::new(self.progress_frac).show_percentage());
                        ui.add_space(8.0);
                        ui.label(&self.status_message);

                        if let Some(ref err) = self.error_message {
                            ui.label(RichText::new(err).color(Color32::from_rgb(231, 76, 60)));
                        } else {
                            // Run the install
                            if let Err(e) = self.perform_installation() {
                                self.error_message = Some(e);
                            } else {
                                self.step = WizardStep::Installed;
                            }
                        }
                    }

                    WizardStep::Installed => {
                        ui.label(
                            RichText::new("✅ Installation Finished Successfully!")
                                .size(16.0)
                                .color(Color32::from_rgb(46, 204, 113))
                                .strong(),
                        );
                        ui.add_space(8.0);
                        ui.label(format!(
                            "OxyDL has been installed to:\n{}",
                            self.install_dir.display()
                        ));
                        ui.add_space(12.0);
                        ui.checkbox(&mut self.launch_after_install, "Launch OxyDL now");

                        ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                            let finish_btn = egui::Button::new(
                                RichText::new("Finish")
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(41, 128, 185));

                            if ui.add(finish_btn).clicked() {
                                if self.launch_after_install {
                                    let exe = self.install_dir.join("oxydl.exe");
                                    let _ = open::that(&exe);
                                }
                                std::process::exit(0);
                            }
                        });
                    }

                    WizardStep::UninstallConfirm => {
                        ui.label(
                            RichText::new("Uninstall OxyDL")
                                .size(16.0)
                                .color(Color32::from_rgb(231, 76, 60))
                                .strong(),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            "Are you sure you want to completely remove OxyDL from your computer?"
                        );
                        ui.add_space(8.0);
                        ui.label(format!("Location: {}", self.install_dir.display()));

                        ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                            ui.horizontal(|ui| {
                                let remove_btn = egui::Button::new(
                                    RichText::new("🗑 Yes, Uninstall")
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(231, 76, 60));

                                if ui.add(remove_btn).clicked() {
                                    self.step = WizardStep::Uninstalling;
                                }
                                if ui.button("Cancel").clicked() {
                                    std::process::exit(0);
                                }
                            });
                        });
                    }

                    WizardStep::Uninstalling => {
                        ui.label(RichText::new("Uninstalling OxyDL...").strong());
                        ui.add_space(8.0);
                        ui.add(ProgressBar::new(self.progress_frac));
                        ui.label(&self.status_message);

                        if let Err(e) = self.perform_uninstallation() {
                            self.error_message = Some(e);
                        } else {
                            self.step = WizardStep::Uninstalled;
                        }
                    }

                    WizardStep::Uninstalled => {
                        ui.label(
                            RichText::new("OxyDL was successfully removed from your computer.")
                                .color(Color32::from_rgb(46, 204, 113))
                                .strong(),
                        );
                        ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                            if ui.button("Close").clicked() {
                                std::process::exit(0);
                            }
                        });
                    }
                }
            });
        });
    }
}

/// Determine default installation directory.
/// On Windows: %LOCALAPPDATA%\Programs\OxyDL
/// On other OS: ~/.local/share/oxydl
fn get_default_install_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(local_app_data).join("Programs").join(APP_NAME);
        }
    }

    if let Some(dirs) = directories::BaseDirs::new() {
        return dirs.data_local_dir().join("Programs").join(APP_NAME);
    }

    PathBuf::from(format!("C:\\Program Files\\{}", APP_NAME))
}

fn get_start_menu_programs_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        if let Ok(app_data) = std::env::var("APPDATA") {
            return Some(
                PathBuf::from(app_data)
                    .join("Microsoft")
                    .join("Windows")
                    .join("Start Menu")
                    .join("Programs"),
            );
        }
    }
    None
}

fn get_desktop_dir() -> Option<PathBuf> {
    if let Some(dirs) = directories::UserDirs::new() {
        if let Some(desktop) = dirs.desktop_dir() {
            return Some(desktop.to_path_buf());
        }
    }
    #[cfg(windows)]
    {
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            return Some(PathBuf::from(userprofile).join("Desktop"));
        }
    }
    None
}

/// Create Windows .lnk shortcut using pure Rust mslnk.
fn create_shortcut(target_exe: &Path, lnk_path: &Path, working_dir: &Path) {
    let link = mslnk::ShellLink::new(target_exe);
    if let Ok(mut l) = link {
        let _ = l.set_working_dir(Some(working_dir.to_string_lossy().to_string()));
        let _ = l.set_icon_location(Some(target_exe.to_string_lossy().to_string()));
        let _ = l.create_lnk(lnk_path);
    }
}

/// Register uninstall info in Windows Registry (HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\OxyDL).
#[cfg(windows)]
fn register_windows_uninstall(install_dir: &Path, target_exe: &Path, uninstaller: &Path) {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let subkey_path = format!(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{}", APP_NAME);

    if let Ok((key, _)) = hkcu.create_subkey(&subkey_path) {
        let _ = key.set_value("DisplayName", &format!("{} - High Speed Download Manager", APP_NAME));
        let _ = key.set_value("DisplayVersion", &APP_VERSION);
        let _ = key.set_value("Publisher", &"OxyDL Open Source Project");
        let _ = key.set_value("InstallLocation", &install_dir.to_string_lossy().to_string());
        let _ = key.set_value("UninstallString", &format!("\"{}\"", uninstaller.to_string_lossy()));
        let _ = key.set_value("DisplayIcon", &format!("\"{}\",0", target_exe.to_string_lossy()));
        let _ = key.set_value("EstimatedSize", &6000u32);
        let _ = key.set_value("NoModify", &1u32);
        let _ = key.set_value("NoRepair", &1u32);
    }
}

#[cfg(not(windows))]
fn register_windows_uninstall(_install_dir: &Path, _target_exe: &Path, _uninstaller: &Path) {}

#[cfg(windows)]
fn unregister_windows_uninstall() {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let subkey_path = format!(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{}", APP_NAME);
    let _ = hkcu.delete_subkey_all(&subkey_path);
}

#[cfg(not(windows))]
fn unregister_windows_uninstall() {}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let current_exe = std::env::current_exe().unwrap_or_default();
    let exe_name = current_exe
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("")
        .to_lowercase();

    let is_uninstall = exe_name.contains("uninstall")
        || args.iter().any(|a| a == "--uninstall" || a == "-u");

    // Headless / Silent automated mode
    if args.iter().any(|a| a == "--silent" || a == "-y") {
        let mut app = InstallerApp::new(is_uninstall);
        if is_uninstall {
            let _ = app.perform_uninstallation();
        } else {
            let _ = app.perform_installation();
        }
        return Ok(());
    }

    // Window icon
    let icon_data = egui::IconData {
        rgba: include_bytes!("../../../assets/icon_32.rgba").to_vec(),
        width: 32,
        height: 32,
    };

    // Interactive GUI Setup Wizard
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(if is_uninstall { "OxyDL Uninstaller" } else { "OxyDL Setup Wizard" })
            .with_icon(std::sync::Arc::new(icon_data))
            .with_inner_size([560.0, 420.0])
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "OxyDL Installer",
        native_options,
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(InstallerApp::new(is_uninstall)))
        }),
    )
}
