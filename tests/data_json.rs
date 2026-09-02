use std::fs;

use video_manager_egui::data_json::{
    add_chapter, add_playlist, load_data, playlist_from_bundle, remove_chapter, save_data,
};
use video_manager_egui::model::{BundleInfo, BundleManifest, Chapter, DataFile};

#[test]
fn default_data_file_has_no_playlists() {
    assert!(DataFile::default().playlists.is_empty());
}

#[test]
fn playlist_mutations_add_and_remove_chapters() {
    let mut data = DataFile::default();
    add_playlist(&mut data, "Training").unwrap();
    add_chapter(
        &mut data,
        0,
        Chapter {
            name: "Pressing".into(),
            start_seconds: 5.0,
            end_seconds: 12.0,
            video_url: "https://cdn.test/pressing.mp4".into(),
            thumbnail_url: String::new(),
        },
    )
    .unwrap();
    assert_eq!(data.playlists[0].chapters.len(), 1);
    remove_chapter(&mut data, 0, 0).unwrap();
    assert!(data.playlists[0].chapters.is_empty());
}

#[test]
fn data_json_round_trips_flutter_video_player_shape() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.json");
    let mut data = DataFile::default();
    add_playlist(&mut data, "R2 clips").unwrap();
    data.playlists[0].video_id = "abc123".into();
    add_chapter(
        &mut data,
        0,
        Chapter {
            name: "Intro".into(),
            start_seconds: 0.0,
            end_seconds: 10.0,
            video_url: "https://cdn.test/intro.mp4".into(),
            thumbnail_url: String::new(),
        },
    )
    .unwrap();
    save_data(&path, &data).unwrap();
    let raw = fs::read_to_string(&path).unwrap();
    assert!(raw.contains("\"type\": \"direct\""));
    assert!(raw.contains("\"videoId\": \"abc123\""));
    assert!(raw.contains("\"startSeconds\": 0.0"));
    assert!(raw.contains("\"videoUrl\": \"https://cdn.test/intro.mp4\""));
    let loaded = load_data(&path).unwrap();
    assert_eq!(loaded, data);
}

#[test]
fn bundle_playlist_uses_uploaded_clip_urls_from_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let video = dir.path().join("video.mp4");
    fs::write(&video, b"fake").unwrap();
    let bundle = BundleInfo {
        dir: dir.path().to_path_buf(),
        title: "Demo".into(),
        source_url: "https://youtu.be/abc".into(),
        video_id: "abc".into(),
        video_path: Some(video),
        transcript_path: None,
        thumbnail_path: None,
        chapters: vec![Chapter {
            name: "One".into(),
            start_seconds: 0.0,
            end_seconds: 10.0,
            video_url: String::new(),
            thumbnail_url: String::new(),
        }],
        manifest: BundleManifest {
            uploads: [(
                "cuts/01-One.mp4".to_string(),
                "https://cdn.test/one.mp4".to_string(),
            )]
            .into_iter()
            .collect(),
            ..BundleManifest::default()
        },
    };
    let playlist = playlist_from_bundle(&bundle);
    assert_eq!(playlist.kind, "direct");
    assert_eq!(playlist.chapters[0].video_url, "https://cdn.test/one.mp4");
}
