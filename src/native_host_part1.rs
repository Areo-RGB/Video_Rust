use std::collections::{BTreeSet, HashSet};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use native_messaging::host::{MAX_FROM_BROWSER, NmError, decode_message_opt, send_json};
use native_messaging::{Scope, install};
use serde_json::{Map, Value, json};

use crate::config::AppConfig;
use crate::local::save_chapter_file;
use crate::model::{Chapter, ChapterFile};
use crate::process::{run_checked, tool_version};
use crate::youtube::{download_bundle, sanitize_filename};

pub const HOST_NAME: &str = "com.fluent_desktop.youtube_clipper";
pub const KNOWN_EXTENSION_ID: &str = "elbdfelfkbnnjojpaoiongmepoohjooi";
const HOST_DESCRIPTION: &str = "Video Manager YouTube Chapter Clipper Native Messaging Host";
const CHROMIUM_BROWSERS: [&str; 5] = ["chrome", "chromium", "edge", "brave", "vivaldi"];
const EXTRA_EXTENSION_IDS_ENV: &str = "VIDEO_MANAGER_NATIVE_EXTENSION_IDS";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RegistrationReport {
    pub installed_browsers: Vec<String>,
    pub errors: Vec<String>,
    pub allowed_origins: Vec<String>,
}

pub trait NativeHostBackend {
    fn ping(&mut self) -> Value;
    fn download_chapter(&mut self, request: &Value) -> Value;
    fn download_full_video(&mut self, request: &Value) -> Value;
    fn open_clip(&mut self, request: &Value) -> Value;

    fn get_jobs(&mut self) -> Value {
        json!({"success": true, "jobs": []})
    }
}

pub fn is_native_host_launch(args: &[OsString]) -> bool {
    args.iter()
        .skip(1)
        .filter_map(|arg| arg.to_str())
        .any(|arg| {
            arg == "--native-messaging-host"
                || arg.starts_with("chrome-extension://")
                || arg.starts_with("extension://")
        })
}

pub fn dispatch_message(backend: &mut impl NativeHostBackend, request: &Value) -> Value {
    let action = request
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("ping");

    let response = match action {
        "ping" => backend.ping(),
        "download_chapter" => backend.download_chapter(request),
        "download_full_video" => backend.download_full_video(request),
        "open_clip" => backend.open_clip(request),
        "get_jobs" => backend.get_jobs(),
        _ => json!({
            "success": false,
            "error": format!("Unknown action: {action}"),
        }),
    };

    with_action(response, action)
}

pub fn run_host_loop_with_io<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    backend: &mut impl NativeHostBackend,
) -> Result<(), NmError> {
    loop {
        let Some(raw) = decode_message_opt(reader, MAX_FROM_BROWSER)? else {
            return Ok(());
        };

        let request = match serde_json::from_str::<Value>(&raw) {
            Ok(request) => request,
            Err(error) => {
                send_json(
                    writer,
                    &json!({
                        "success": false,
                        "error": format!("Invalid JSON request: {error}"),
                    }),
                )?;
                continue;
            }
        };
        let response = dispatch_message(backend, &request);
        send_json(writer, &response)?;
    }
}

pub fn run_native_host_stdio() -> Result<(), NmError> {
    let mut backend = AppNativeHostBackend::new(AppConfig::load());
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();
    run_host_loop_with_io(&mut reader, &mut writer, &mut backend)
}

pub fn auto_register_current_executable() -> RegistrationReport {
    match env::current_exe() {
        Ok(exe) => register_executable(&exe),
        Err(error) => RegistrationReport {
            errors: vec![format!("could not resolve current executable: {error}")],
            ..RegistrationReport::default()
        },
    }
}

pub fn register_executable(exe_path: &Path) -> RegistrationReport {
    let mut ids = discover_extension_ids();
    ids.push(KNOWN_EXTENSION_ID.to_owned());
    if let Some(extra) = env::var_os(EXTRA_EXTENSION_IDS_ENV) {
        ids.extend(
            extra
                .to_string_lossy()
                .split([',', ';', ' ', '\t', '\n'])
                .filter(|value| !value.trim().is_empty())
                .map(|value| value.trim().to_owned()),
        );
    }
    let allowed_origins = allowed_origins_from_ids(ids);
    let mut report = RegistrationReport {
        allowed_origins: allowed_origins.clone(),
        ..RegistrationReport::default()
    };

    for browser in CHROMIUM_BROWSERS {
        match install(
            HOST_NAME,
            HOST_DESCRIPTION,
            exe_path,
            &allowed_origins,
            &[],
            &[browser],
            Scope::User,
        ) {
            Ok(()) => report.installed_browsers.push(browser.to_owned()),
            Err(error) => report.errors.push(format!("{browser}: {error}")),
        }
    }

    report
}

pub fn allowed_origins_from_ids(ids: impl IntoIterator<Item = String>) -> Vec<String> {
    ids.into_iter()
        .filter(|id| is_valid_extension_id(id))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|id| format!("chrome-extension://{id}/"))
        .collect()
}

pub fn discover_extension_ids() -> Vec<String> {
    discover_extension_ids_in_roots(&browser_profile_roots())
}

pub fn discover_extension_ids_in_roots(roots: &[PathBuf]) -> Vec<String> {
    let mut ids = BTreeSet::new();
    for root in roots {
        for profile in profile_candidates(root) {
            collect_ids_from_extensions_dir(&profile.join("Extensions"), &mut ids);
            collect_ids_from_preferences(&profile.join("Preferences"), &mut ids);
        }
    }
    ids.into_iter().collect()
}

pub fn chapter_download_args(
    config: &AppConfig,
    output_path: &Path,
    url: &str,
    start_seconds: f64,
    end_seconds: f64,
) -> Vec<String> {
    let mut args = vec![
        "--no-playlist".into(),
        "--no-warnings".into(),
        "--merge-output-format".into(),
        "mp4".into(),
    ];
    if !config.ffmpeg_path.trim().is_empty() {
        args.push("--ffmpeg-location".into());
        args.push(config.ffmpeg_path.clone());
    }

    let section = if end_seconds > start_seconds {
        Some(format!(
            "*{}-{}",
            cli_seconds(start_seconds),
            cli_seconds(end_seconds)
        ))
    } else if start_seconds > 0.0 {
        Some(format!("*{}-inf", cli_seconds(start_seconds)))
    } else {
        None
    };
    if let Some(section) = section {
        args.push("--download-sections".into());
        args.push(section);
        args.push("--force-keyframes-at-cuts".into());
    }

    args.push("-o".into());
    args.push(output_path.to_string_lossy().into_owned());
    args.push(url.to_owned());
    args
}
