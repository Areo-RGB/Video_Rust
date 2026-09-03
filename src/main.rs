#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::env;

use video_manager_egui::app::VideoManagerApp;
use video_manager_egui::native_host::{
    auto_register_current_executable, is_native_host_launch, run_native_host_stdio,
};

fn main() -> eframe::Result<()> {
    let args = env::args_os().collect::<Vec<_>>();
    if is_native_host_launch(&args) {
        if let Err(error) = run_native_host_stdio() {
            eprintln!("native messaging host failed: {error}");
            std::process::exit(1);
        }
        return Ok(());
    }

    let registration = auto_register_current_executable();
    for error in registration.errors {
        eprintln!("native messaging registration failed: {error}");
    }

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1220.0, 780.0])
            .with_min_inner_size([900.0, 600.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "Video Manager — Rust + egui",
        options,
        Box::new(|cc| Ok(Box::new(VideoManagerApp::new(cc)))),
    )
}
