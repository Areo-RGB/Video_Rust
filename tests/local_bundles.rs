use std::fs;

use video_manager_egui::local::{
    load_bundle, record_uploaded_url, save_chapter_file, scan_bundles,
};
use video_manager_egui::model::{Chapter, ChapterFile};

#[test]
fn scans_workspace_for_video_bundles() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("Demo");
    fs::create_dir_all(&bundle).unwrap();
    fs::write(bundle.join("video.mp4"), b"fake").unwrap();
    save_chapter_file(
        &bundle,
        &ChapterFile {
            video_id: "abc".into(),
            video_title: "Demo".into(),
            url: "https://youtu.be/abc".into(),
            chapters: vec![Chapter {
                name: "One".into(),
                start_seconds: 0.0,
                end_seconds: 10.0,
                video_url: String::new(),
                thumbnail_url: String::new(),
            }],
            available: true,
        },
    )
    .unwrap();
    let bundles = scan_bundles(dir.path()).unwrap();
    assert_eq!(bundles.len(), 1);
    assert_eq!(bundles[0].file_name().unwrap().to_string_lossy(), "Demo");
    assert_eq!(load_bundle(&bundles[0]).unwrap().chapters.len(), 1);
}

#[test]
fn records_uploaded_url_in_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let manifest =
        record_uploaded_url(dir.path(), "cuts/one.mp4", "https://cdn.test/one.mp4").unwrap();
    assert_eq!(manifest.uploads["cuts/one.mp4"], "https://cdn.test/one.mp4");
}
