//! Oxidm - Open-source, high-speed multi-connection download manager in pure Rust.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use oxidm::app::OxidmApp;

fn main() -> eframe::Result<()> {
    // Check command line arguments for CLI mode or direct URL launch
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && (args[1] == "--help" || args[1] == "-h") {
        println!("Oxidm - High Speed Download Manager (Pure Rust)");
        println!("Usage:");
        println!("  oxidm                 Launch the graphical user interface");
        println!("  oxidm <url>           Launch GUI and automatically queue <url>");
        println!("  oxidm --version       Display version information");
        return Ok(());
    }

    if args.len() > 1 && (args[1] == "--version" || args[1] == "-v") {
        println!("Oxidm v{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // Initialize Tokio asynchronous runtime
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to initialize Tokio runtime");

    let _guard = rt.enter();

    // Configure native window options
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Oxidm - High Speed Download Manager")
            .with_inner_size([1020.0, 680.0])
            .with_min_inner_size([760.0, 480.0])
            .with_active(true),
        ..Default::default()
    };

    eframe::run_native(
        "Oxidm",
        native_options,
        Box::new(|cc| {
            // Apply modern dark theme with clean crisp font rendering
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(OxidmApp::new(cc)))
        }),
    )
}
