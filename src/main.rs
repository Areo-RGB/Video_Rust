#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use video_manager_egui::app::VideoManagerApp;

fn main() -> eframe::Result<()> {
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
