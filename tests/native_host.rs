use std::ffi::OsString;
use std::fs;
use std::io::Cursor;

use serde_json::{Value, json};
use tempfile::tempdir;
use video_manager_egui::native_host::{
    HOST_NAME, KNOWN_EXTENSION_ID, NativeHostBackend, allowed_origins_from_ids,
    discover_extension_ids_in_roots, dispatch_message, is_native_host_launch, run_host_loop_with_io,
};

#[derive(Default)]
struct StubBackend;

impl NativeHostBackend for StubBackend {
    fn ping(&mut self) -> Value {
        json!({
            "success": true,
            "action": "ping",
            "host": HOST_NAME,
            "versions": {"yt_dlp_version": "test", "ffmpeg_version": "test"},
            "outputDir": "/tmp/clips"
        })
    }

    fn download_chapter(&mut self, request: &Value) -> Value {
        json!({
            "success": true,
            "jobId": "job_test",
            "outputPath": "/tmp/chapter.mp4",
            "chapterTitle": request.get("chapterTitle").and_then(Value::as_str).unwrap_or_default()
        })
    }

    fn download_full_video(&mut self, _request: &Value) -> Value {
        json!({"success": true, "jobId": "full_test", "bundlePath": "/tmp/bundle"})
    }

    fn open_clip(&mut self, _request: &Value) -> Value {
        json!({"success": true, "action": "open_clip"})
    }
}

#[test]
fn detects_browser_native_host_launch_from_extension_origin() {
    let args = vec![
        OsString::from("video-manager-egui"),
        OsString::from("chrome-extension://abcdefghijklmnopabcdefghijklmnop/"),
    ];
    assert!(is_native_host_launch(&args));

    let normal = vec![OsString::from("video-manager-egui")];
    assert!(!is_native_host_launch(&normal));
}

#[test]
fn allowed_origins_include_known_and_discovered_extensions() {
    let origins = allowed_origins_from_ids([
        KNOWN_EXTENSION_ID.to_string(),
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
    ]);
    assert_eq!(origins[0], "chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/");
    assert!(origins.contains(&format!("chrome-extension://{KNOWN_EXTENSION_ID}/")));
}

#[test]
fn discovers_extension_ids_from_preferences_and_extensions_directories() {
    let root = tempdir().unwrap();
    let profile = root.path().join("Default");
    fs::create_dir_all(profile.join("Extensions").join("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")).unwrap();
    fs::write(
        profile.join("Preferences"),
        r#"{"extensions":{"settings":{"cccccccccccccccccccccccccccccccc":{},"INVALID":{}}}}"#,
    )
    .unwrap();

    let ids = discover_extension_ids_in_roots(&[root.path().to_path_buf()]);
    assert!(ids.contains(&"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string()));
    assert!(ids.contains(&"cccccccccccccccccccccccccccccccc".to_string()));
    assert!(!ids.contains(&"INVALID".to_string()));
}

#[test]
fn dispatch_preserves_extension_response_contract() {
    let mut backend = StubBackend;
    let ping = dispatch_message(&mut backend, &json!({"action":"ping"}));
    assert_eq!(ping["success"], true);
    assert_eq!(ping["action"], "ping");
    assert_eq!(ping["host"], HOST_NAME);

    let clip = dispatch_message(
        &mut backend,
        &json!({"action":"download_chapter", "chapterTitle":"Pressing"}),
    );
    assert_eq!(clip["success"], true);
    assert_eq!(clip["jobId"], "job_test");
    assert_eq!(clip["chapterTitle"], "Pressing");
    assert_eq!(clip["action"], "download_chapter");

    let unknown = dispatch_message(&mut backend, &json!({"action":"anything_goes"}));
    assert_eq!(unknown["success"], false);
    assert!(unknown["error"].as_str().unwrap().contains("Unknown action"));
}

#[test]
fn host_loop_reads_and_writes_multiple_framed_messages() {
    let mut input = Vec::new();
    input.extend(native_messaging::host::encode_message(&json!({"action":"ping"})).unwrap());
    input.extend(
        native_messaging::host::encode_message(&json!({"action":"download_full_video"})).unwrap(),
    );

    let mut reader = Cursor::new(input);
    let mut output = Vec::new();
    let mut backend = StubBackend;
    run_host_loop_with_io(&mut reader, &mut output, &mut backend).unwrap();

    let mut replies = Cursor::new(output);
    let first = native_messaging::host::decode_message(
        &mut replies,
        native_messaging::host::MAX_FROM_BROWSER,
    )
    .unwrap();
    let second = native_messaging::host::decode_message(
        &mut replies,
        native_messaging::host::MAX_FROM_BROWSER,
    )
    .unwrap();
    let first: Value = serde_json::from_str(&first).unwrap();
    let second: Value = serde_json::from_str(&second).unwrap();
    assert_eq!(first["action"], "ping");
    assert_eq!(second["action"], "download_full_video");
}
