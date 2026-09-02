use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct R2Config {
    #[serde(default)]
    pub account_id: String,
    #[serde(default)]
    pub bucket: String,
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub public_base_url: String,
    #[serde(default)]
    pub access_key_id: String,
    #[serde(default)]
    pub secret_access_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub workspace_dir: String,
    pub data_json_path: String,
    pub yt_dlp_path: String,
    pub ffmpeg_path: String,
    pub r2: R2Config,
}

impl Default for AppConfig {
    fn default() -> Self {
        let base = dirs::video_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("VideoManager");
        Self {
            workspace_dir: base.to_string_lossy().into_owned(),
            data_json_path: base.join("data.json").to_string_lossy().into_owned(),
            yt_dlp_path: "yt-dlp".into(),
            ffmpeg_path: "ffmpeg".into(),
            r2: R2Config::default(),
        }
    }
}

impl AppConfig {
    pub fn settings_path() -> PathBuf {
        dirs::config_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("video-manager-egui")
            .join("settings.json")
    }

    pub fn load() -> Self {
        let path = Self::settings_path();
        let mut cfg = fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<Self>(&raw).ok())
            .unwrap_or_default();

        let mut vars: BTreeMap<String, String> = env::vars().collect();
        for dotenv in dotenv_candidates() {
            if let Ok(from_file) = read_dotenv(&dotenv) {
                for (key, value) in from_file {
                    vars.entry(key).or_insert(value);
                }
            }
        }
        cfg.apply_env_map(&vars);
        cfg
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::settings_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(&path, format!("{json}\n")).map_err(|e| AppError::io(path, e))
    }

    pub fn apply_env_map(&mut self, vars: &BTreeMap<String, String>) {
        set_if_present(&mut self.workspace_dir, vars, "VIDEO_MANAGER_WORKSPACE");
        set_if_present(&mut self.data_json_path, vars, "VIDEO_MANAGER_DATA_JSON");
        set_if_present(&mut self.yt_dlp_path, vars, "YT_DLP_PATH");
        set_if_present(&mut self.ffmpeg_path, vars, "FFMPEG_PATH");
        set_if_present(&mut self.r2.account_id, vars, "R2_ACCOUNT_ID");
        set_if_present(&mut self.r2.bucket, vars, "R2_BUCKET");
        set_if_present(&mut self.r2.prefix, vars, "R2_PREFIX");
        set_if_present(&mut self.r2.public_base_url, vars, "R2_PUBLIC_BASE_URL");
        set_if_present(&mut self.r2.access_key_id, vars, "R2_ACCESS_KEY_ID");
        set_if_present(&mut self.r2.secret_access_key, vars, "R2_SECRET_ACCESS_KEY");
    }
}

fn set_if_present(target: &mut String, vars: &BTreeMap<String, String>, key: &str) {
    if let Some(value) = vars.get(key).filter(|value| !value.trim().is_empty()) {
        *target = value.clone();
    }
}

fn dotenv_candidates() -> Vec<PathBuf> {
    let mut result = Vec::new();
    if let Ok(current) = env::current_dir() {
        result.push(current.join(".env"));
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            result.push(parent.join(".env"));
        }
    }
    result.sort();
    result.dedup();
    result
}

fn read_dotenv(path: &Path) -> Result<BTreeMap<String, String>> {
    let raw = fs::read_to_string(path).map_err(|e| AppError::io(path, e))?;
    let mut result = BTreeMap::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value
            .trim()
            .trim_matches(|c| c == '"' || c == '\'')
            .to_owned();
        result.insert(key.trim().to_owned(), value);
    }
    Ok(result)
}
