use video_manager_egui::youtube::{parse_metadata_json, sanitize_filename, vtt_to_text};

#[test]
fn sanitizes_cross_platform_filename_characters() {
    assert_eq!(sanitize_filename("  A/B:*?  Clip  "), "A_B___ Clip");
}

#[test]
fn parses_yt_dlp_chapters_and_fills_last_end_from_duration() {
    let raw = r#"{"id":"abc","title":"Demo","webpage_url":"https://youtu.be/abc","duration":120.0,"chapters":[{"title":"One","start_time":0.0,"end_time":30.0},{"title":"Two","start_time":30.0}]}"#;
    let meta = parse_metadata_json(raw).unwrap();
    assert_eq!(meta.id, "abc");
    assert_eq!(meta.chapters.len(), 2);
    assert_eq!(meta.chapters[1].end_seconds, 120.0);
}

#[test]
fn vtt_conversion_keeps_timestamps_and_deduplicates_lines() {
    let input = "WEBVTT\n\n00:00:01.000 --> 00:00:03.000\nHello world\n\n00:00:03.000 --> 00:00:05.000\nHello world\n\n00:00:05.000 --> 00:00:06.000\nNext line\n";
    let output = vtt_to_text(input);
    assert!(output.contains("[00:00:01.000 --> 00:00:03.000] Hello world"));
    assert!(output.contains("[00:00:05.000 --> 00:00:06.000] Next line"));
    assert_eq!(output.matches("Hello world").count(), 1);
}
