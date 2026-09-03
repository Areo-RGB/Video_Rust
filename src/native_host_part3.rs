fn with_action(mut response: Value, action: &str) -> Value {
    if let Some(object) = response.as_object_mut() {
        object
            .entry("action".to_owned())
            .or_insert_with(|| Value::String(action.to_owned()));
        response
    } else {
        json!({
            "success": false,
            "action": action,
            "error": "Native host backend returned a non-object response",
        })
    }
}

fn json_error(message: impl Into<String>) -> Value {
    json!({"success": false, "error": message.into()})
}

fn request_string(request: &Value, key: &str) -> Option<String> {
    request
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .map(str::to_owned)
}

fn request_number(request: &Value, key: &str) -> Option<f64> {
    value_as_f64(request.get(key)?)
}

fn value_as_f64(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|text| text.trim().parse().ok()))
}

fn string_alias(object: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        object
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .map(str::to_owned)
    })
}

fn number_alias(object: &Map<String, Value>, keys: &[&str]) -> Option<f64> {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(value_as_f64))
}

fn nonempty_or(value: Option<String>, fallback: &str) -> String {
    value
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| fallback.to_owned())
}

fn cli_seconds(value: f64) -> String {
    if value.fract() == 0.0 {
        return format!("{value:.0}");
    }
    let mut result = format!("{value:.3}");
    while result.ends_with('0') {
        result.pop();
    }
    if result.ends_with('.') {
        result.pop();
    }
    result
}

fn format_timestamp(seconds: f64) -> String {
    let total = seconds.max(0.0).floor() as u64;
    let hours = total / 3600;
    let minutes = (total / 60) % 60;
    let seconds = total % 60;
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

fn unix_seconds() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64())
        .unwrap_or_default()
}

fn make_job_id(prefix: &str) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    format!("{prefix}_{millis}")
}

fn is_valid_extension_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|byte| (b'a'..=b'p').contains(&byte))
}

fn profile_candidates(root: &Path) -> Vec<PathBuf> {
    let mut profiles = vec![root.to_path_buf()];
    let Ok(entries) = fs::read_dir(root) else {
        return profiles;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && (path.join("Preferences").is_file() || path.join("Extensions").is_dir())
        {
            profiles.push(path);
        }
    }
    profiles
}

fn collect_ids_from_extensions_dir(dir: &Path, ids: &mut BTreeSet<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.path().is_dir() && is_valid_extension_id(&name) {
            ids.insert(name);
        }
    }
}

fn collect_ids_from_preferences(path: &Path, ids: &mut BTreeSet<String>) {
    let Ok(raw) = fs::read_to_string(path) else {
        return;
    };
    let Ok(value) = serde_json::from_str::<Value>(&raw) else {
        return;
    };
    let Some(settings) = value
        .get("extensions")
        .and_then(|value| value.get("settings"))
        .and_then(Value::as_object)
    else {
        return;
    };
    for id in settings.keys() {
        if is_valid_extension_id(id) {
            ids.insert(id.clone());
        }
    }
}

fn browser_profile_roots() -> Vec<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        let Some(home) = dirs::home_dir() else {
            return Vec::new();
        };
        return [
            ".config/google-chrome",
            ".config/google-chrome-beta",
            ".config/google-chrome-unstable",
            ".config/chromium",
            ".config/BraveSoftware/Brave-Browser",
            ".config/microsoft-edge",
            ".config/microsoft-edge-beta",
            ".config/vivaldi",
            ".var/app/com.google.Chrome/config/google-chrome",
            ".var/app/com.google.ChromeDev/config/google-chrome-unstable",
            ".var/app/org.chromium.Chromium/config/chromium",
        ]
        .into_iter()
        .map(|relative| home.join(relative))
        .collect();
    }

    #[cfg(target_os = "windows")]
    {
        let Some(local) = env::var_os("LOCALAPPDATA").map(PathBuf::from) else {
            return Vec::new();
        };
        return [
            "Google/Chrome/User Data",
            "Chromium/User Data",
            "BraveSoftware/Brave-Browser/User Data",
            "Microsoft/Edge/User Data",
            "Vivaldi/User Data",
        ]
        .into_iter()
        .map(|relative| local.join(relative))
        .collect();
    }

    #[cfg(target_os = "macos")]
    {
        let Some(home) = dirs::home_dir() else {
            return Vec::new();
        };
        return [
            "Library/Application Support/Google/Chrome",
            "Library/Application Support/Chromium",
            "Library/Application Support/BraveSoftware/Brave-Browser",
            "Library/Application Support/Microsoft Edge",
            "Library/Application Support/Vivaldi",
        ]
        .into_iter()
        .map(|relative| home.join(relative))
        .collect();
    }

    #[allow(unreachable_code)]
    Vec::new()
}

fn open_path(path: &Path) -> io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(path).spawn()?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(path).spawn()?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("rundll32.exe")
            .arg("url.dll,FileProtocolHandler")
            .arg(path)
            .spawn()?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "opening files is not supported on this platform",
    ))
}
