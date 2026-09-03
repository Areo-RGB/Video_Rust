use std::path::Path;

use video_manager_egui::ui_helpers::{
    LocalLayoutMode, local_file_uri, local_layout_mode, matches_local_filter,
};

#[test]
fn local_layout_uses_three_panes_when_desktop_width_allows_it() {
    assert_eq!(local_layout_mode(1100.0), LocalLayoutMode::ThreePane);
    assert_eq!(local_layout_mode(900.0), LocalLayoutMode::ThreePane);
    assert_eq!(local_layout_mode(899.0), LocalLayoutMode::Compact);
}

#[test]
fn local_file_uri_supports_unix_and_windows_paths() {
    assert_eq!(
        local_file_uri(Path::new("/home/user/Videos/thumb.webp")),
        "file:///home/user/Videos/thumb.webp"
    );
    assert_eq!(
        local_file_uri(Path::new(r"C:\Users\user\Videos\thumb.webp")),
        "file:///C:/Users/user/Videos/thumb.webp"
    );
}

#[test]
fn local_filter_is_case_insensitive_and_searches_secondary_text() {
    assert!(matches_local_filter("01-Intro.mp4", "https://media.example/Intro.mp4", "intro"));
    assert!(matches_local_filter("01-Intro.mp4", "https://media.example/Intro.mp4", "MEDIA.EXAMPLE"));
    assert!(matches_local_filter("anything", "else", "   "));
    assert!(!matches_local_filter("01-Intro.mp4", "https://media.example/Intro.mp4", "drill"));
}
