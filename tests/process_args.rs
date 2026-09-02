use std::path::Path;
use video_manager_egui::config::AppConfig;
use video_manager_egui::local::cut_args;
use video_manager_egui::model::Chapter;
use video_manager_egui::youtube::{download_video_args, metadata_args};

#[test]
fn metadata_command_skips_download() {
    let args = metadata_args("https://youtu.be/abc");
    assert!(args.contains(&"--skip-download".to_string()));
    assert!(args.contains(&"--dump-single-json".to_string()));
}

#[test]
fn download_command_uses_configured_ffmpeg() {
    let cfg = AppConfig {
        ffmpeg_path: "/tools/ffmpeg".into(),
        ..Default::default()
    };
    let args = download_video_args(&cfg, Path::new("video.mp4"), "https://youtu.be/abc");
    assert!(
        args.windows(2)
            .any(|w| w[0] == "--ffmpeg-location" && w[1] == "/tools/ffmpeg")
    );
}

#[test]
fn cut_command_reencodes_for_accurate_boundaries() {
    let chapter = Chapter {
        name: "One".into(),
        start_seconds: 12.5,
        end_seconds: 20.0,
        video_url: String::new(),
        thumbnail_url: String::new(),
    };
    let (_, args) = cut_args(
        "ffmpeg",
        Path::new("video.mp4"),
        &chapter,
        Path::new("cut.mp4"),
    );
    assert!(args.windows(2).any(|w| w[0] == "-c:v" && w[1] == "libx264"));
    assert!(args.windows(2).any(|w| w[0] == "-t" && w[1] == "7.500"));
}
