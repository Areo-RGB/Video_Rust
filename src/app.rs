use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui;

use crate::config::{AppConfig, DATA_JSON_RAW_URL, DATA_JSON_SOURCE_URL};
use crate::data_json::{
    add_chapter, add_playlist, fetch_data, remove_chapter, remove_playlist, upsert_bundle_playlist,
};
use crate::jobs::{JobMessage, JobValue, spawn_job};
use crate::local::{
    bundle_object_name, cut_all, cut_chapter, cut_relative_name, load_bundle, record_uploaded_url,
    scan_bundles,
};
use crate::model::{BundleInfo, Chapter, DataFile, R2Object, VideoMetadata};
use crate::pipeline::cut_and_upload;
use crate::process::tool_version;
use crate::r2::R2Client;
use crate::youtube::{download_bundle, fetch_metadata};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Tab {
    #[default]
    Youtube,
    Local,
    R2,
    Data,
    Settings,
    Log,
}

impl Tab {
    fn label(self) -> &'static str {
        match self {
            Self::Youtube => "YouTube",
            Self::Local => "Local Files",
            Self::R2 => "R2",
            Self::Data => "Data JSON",
            Self::Settings => "Settings",
            Self::Log => "Log",
        }
    }

    fn all() -> [Self; 6] {
        [
            Self::Youtube,
            Self::Local,
            Self::R2,
            Self::Data,
            Self::Settings,
            Self::Log,
        ]
    }
}

pub struct VideoManagerApp {
    config: AppConfig,
    tab: Tab,
    youtube_url: String,
    metadata: Option<VideoMetadata>,
    bundles: Vec<PathBuf>,
    selected_bundle: Option<BundleInfo>,
    local_file_filter: String,
    r2_objects: Vec<R2Object>,
    data: DataFile,
    data_path: String,
    selected_playlist: Option<usize>,
    new_playlist: String,
    new_chapter_name: String,
    new_chapter_start: f64,
    new_chapter_end: f64,
    new_chapter_url: String,
    last_uploaded_url: String,
    logs: Vec<String>,
    status: String,
    active_jobs: usize,
    job_tx: Sender<JobMessage>,
    job_rx: Receiver<JobMessage>,
    yt_version: Option<String>,
    ffmpeg_version: Option<String>,
}

impl Default for VideoManagerApp {
    fn default() -> Self {
        let config = AppConfig::load();
        let data_path = DATA_JSON_SOURCE_URL.to_owned();
        let bundles = scan_bundles(Path::new(&config.workspace_dir)).unwrap_or_default();
        let (job_tx, job_rx) = channel();
        let yt_version = tool_version(&config.yt_dlp_path, "--version");
        let ffmpeg_version = tool_version(&config.ffmpeg_path, "-version");
        Self {
            config,
            tab: Tab::Youtube,
            youtube_url: String::new(),
            metadata: None,
            bundles,
            selected_bundle: None,
            local_file_filter: String::new(),
            r2_objects: Vec::new(),
            data: DataFile::default(),
            data_path,
            selected_playlist: None,
            new_playlist: String::new(),
            new_chapter_name: String::new(),
            new_chapter_start: 0.0,
            new_chapter_end: 0.0,
            new_chapter_url: String::new(),
            last_uploaded_url: String::new(),
            logs: vec!["Video Manager started".into()],
            status: "Ready".into(),
            active_jobs: 0,
            job_tx,
            job_rx,
            yt_version,
            ffmpeg_version,
        }
    }
}

include!("app_widgets.rs");
include!("app_core.rs");
include!("app_youtube.rs");
include!("app_local_ui.rs");
include!("app_r2_ui.rs");
include!("app_data_ui.rs");
include!("app_settings_ui.rs");

impl eframe::App for VideoManagerApp {
    fn ui(&mut self, root_ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_jobs();
        self.nav(root_ui);
        egui::CentralPanel::default().show(root_ui, |ui| match self.tab {
            Tab::Youtube => self.ui_youtube(ui),
            Tab::Local => self.ui_local(ui),
            Tab::R2 => self.ui_r2(ui),
            Tab::Data => self.ui_data(ui),
            Tab::Settings => self.ui_settings(ui),
            Tab::Log => self.ui_log(ui),
        });
        if self.active_jobs > 0 {
            root_ui
                .ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_starts_on_youtube_tab() {
        let app = VideoManagerApp::default();
        assert_eq!(app.tab, Tab::Youtube);
    }
}
