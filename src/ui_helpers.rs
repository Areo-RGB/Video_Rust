use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalLayoutMode {
    ThreePane,
    Compact,
}

pub fn local_layout_mode(available_width: f32) -> LocalLayoutMode {
    if available_width >= 900.0 {
        LocalLayoutMode::ThreePane
    } else {
        LocalLayoutMode::Compact
    }
}

pub fn local_file_uri(path: &Path) -> String {
    let raw = path.to_string_lossy().replace('\\', "/");
    if raw.starts_with("http://") || raw.starts_with("https://") {
        return raw;
    }
    let normalized = if raw.starts_with('/') {
        raw
    } else {
        format!("/{raw}")
    };
    format!("file://{}", percent_encode_path(&normalized))
}

pub fn matches_local_filter(primary: &str, secondary: &str, query: &str) -> bool {
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return true;
    }
    primary.to_ascii_lowercase().contains(&query) || secondary.to_ascii_lowercase().contains(&query)
}

fn percent_encode_path(path: &str) -> String {
    let mut encoded = String::with_capacity(path.len());
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b':' | b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            use std::fmt::Write as _;
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}
