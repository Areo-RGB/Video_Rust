use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::config::AppConfig;
use crate::error::{AppError, Result};
use crate::local::{save_chapter_file, save_manifest};
use crate::model::{BundleInfo, BundleManifest, Chapter, ChapterFile, VideoMetadata};
use crate::process::{run_capture, run_checked};

#[derive(Debug, Deserialize)]
struct RawChapter {
    #[serde(default)]
    title: String,
    #[serde(default)]
    start_time: f64,
    end_time: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct RawMetadata {
    #[serde(default)]
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    webpage_url: String,
    #[serde(default)]
    duration: f64,
    #[serde(default)]
    chapters: Vec<RawChapter>,
}

pub fn sanitize_filename(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut previous_space = false;
    for ch in input.trim().chars() {
        let replacement = if matches!(ch, '\\' | '/' | '*' | '?' | ':' | '"' | '<' | '>' | '|') {
            '_'
        } else {
            ch
        };
        if replacement.is_whitespace() {
            if !previous_space {
                out.push(' ');
            }
            previous_space = true;
        } else {
            out.push(replacement);
            previous_space = false;
        }
        if out.len() >= 120 {
            break;
        }
    }
    let trimmed = out.trim_matches(|c: char| c == ' ' || c == '.').to_owned();
    if trimmed.is_empty() {
        "video".into()
    } else {
        trimmed
    }
}

pub fn parse_metadata_json(raw: &str) -> Result<VideoMetadata> {
    let meta: RawMetadata = serde_json::from_str(raw)?;
    if meta.id.is_empty() {
        return Err(AppError::Invalid(
            "yt-dlp metadata did not include a video id".into(),
        ));
    }
    let title = if meta.title.trim().is_empty() {
        meta.id.clone()
    } else {
        meta.title
    };
    let mut chapters = Vec::with_capacity(meta.chapters.len());
    for (index, chapter) in meta.chapters.iter().enumerate() {
        let end = chapter
            .end_time
            .or_else(|| meta.chapters.get(index + 1).map(|next| next.start_time))
            .unwrap_or(meta.duration);
        if end > chapter.start_time {
            chapters.push(Chapter {
                name: if chapter.title.trim().is_empty() {
                    format!("Chapter {}", index + 1)
                } else {
                    chapter.title.trim().to_owned()
                },
                start_seconds: chapter.start_time.max(0.0),
                end_seconds: end,
                video_url: String::new(),
                thumbnail_url: String::new(),
            });
        }
    }
    Ok(VideoMetadata {
        id: meta.id.clone(),
        title,
        url: if meta.webpage_url.is_empty() {
            format!("https://www.youtube.com/watch?v={}", meta.id)
        } else {
            meta.webpage_url
        },
        chapters,
        duration: meta.duration,
    })
}

pub fn vtt_to_text(vtt: &str) -> String {
    let mut lines = vtt.lines().peekable();
    let mut output = Vec::new();
    let mut previous_text = String::new();

    while let Some(line) = lines.next() {
        let line = line.trim();
        if line.is_empty()
            || line == "WEBVTT"
            || line.starts_with("Kind:")
            || line.starts_with("Language:")
        {
            continue;
        }
        let timing = if line.contains("-->") {
            line.to_owned()
        } else if lines.peek().is_some_and(|next| next.contains("-->")) {
            lines.next().unwrap().trim().to_owned()
        } else {
            continue;
        };

        let mut cue_parts = Vec::new();
        while let Some(next) = lines.peek() {
            let next = next.trim();
            if next.is_empty() {
                lines.next();
                break;
            }
            if next.contains("-->") {
                break;
            }
            cue_parts.push(strip_vtt_tags(next));
            lines.next();
        }
        let text = cue_parts
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if !text.is_empty() && text != previous_text {
            output.push(format!("[{timing}] {text}"));
            previous_text = text;
        }
    }
    if output.is_empty() {
        String::new()
    } else {
        format!("{}\n", output.join("\n"))
    }
}

fn strip_vtt_tags(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

pub fn metadata_args(url: &str) -> Vec<String> {
    vec![
        "--no-playlist".into(),
        "--no-warnings".into(),
        "--dump-single-json".into(),
        "--skip-download".into(),
        url.into(),
    ]
}

pub fn download_video_args(config: &AppConfig, output: &Path, url: &str) -> Vec<String> {
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
    args.extend([
        "-o".into(),
        output.to_string_lossy().into_owned(),
        url.into(),
    ]);
    args
}

pub fn subtitle_args(output_template: &Path, url: &str) -> Vec<String> {
    vec![
        "--no-playlist".into(),
        "--no-warnings".into(),
        "--skip-download".into(),
        "--write-auto-subs".into(),
        "--write-subs".into(),
        "--write-thumbnail".into(),
        "--sub-langs".into(),
        "de.*,en.*,de,en".into(),
        "--sub-format".into(),
        "vtt".into(),
        "-o".into(),
        output_template.to_string_lossy().into_owned(),
        url.into(),
    ]
}

pub fn fetch_metadata(config: &AppConfig, url: &str) -> Result<VideoMetadata> {
    let output = run_capture(&config.yt_dlp_path, &metadata_args(url))?;
    parse_metadata_json(&output)
}

pub fn download_bundle(config: &AppConfig, url: &str) -> Result<BundleInfo> {
    let metadata = fetch_metadata(config, url)?;
    let bundle_dir = PathBuf::from(&config.workspace_dir).join(sanitize_filename(&metadata.title));
    fs::create_dir_all(&bundle_dir).map_err(|e| AppError::io(&bundle_dir, e))?;

    let video_path = bundle_dir.join("video.mp4");
    run_checked(
        &config.yt_dlp_path,
        &download_video_args(config, &video_path, &metadata.url),
    )?;

    let chapter_file = ChapterFile {
        video_id: metadata.id.clone(),
        video_title: metadata.title.clone(),
        url: metadata.url.clone(),
        chapters: metadata.chapters.clone(),
        available: !metadata.chapters.is_empty(),
    };
    save_chapter_file(&bundle_dir, &chapter_file)?;

    let subs_dir = bundle_dir.join(".subs");
    fs::create_dir_all(&subs_dir).map_err(|e| AppError::io(&subs_dir, e))?;
    let template = subs_dir.join("%(id)s.%(ext)s");
    let _ = run_checked(
        &config.yt_dlp_path,
        &subtitle_args(&template, &metadata.url),
    );

    let mut transcript_path = None;
    if let Some(vtt) = find_first_extension(&subs_dir, &["vtt"])? {
        let raw = fs::read_to_string(&vtt).map_err(|e| AppError::io(&vtt, e))?;
        let transcript = vtt_to_text(&raw);
        if !transcript.is_empty() {
            let path = bundle_dir.join("transcript.txt");
            fs::write(&path, transcript).map_err(|e| AppError::io(&path, e))?;
            transcript_path = Some(path);
        }
    }

    let mut thumbnail_path = None;
    if let Some(image) = find_first_extension(&subs_dir, &["jpg", "jpeg", "png", "webp"])? {
        let ext = image.extension().and_then(|e| e.to_str()).unwrap_or("jpg");
        let target = bundle_dir.join(format!("thumbnail.{ext}"));
        fs::copy(&image, &target).map_err(|e| AppError::io(&target, e))?;
        thumbnail_path = Some(target);
    }
    let _ = fs::remove_dir_all(&subs_dir);

    let mut manifest = BundleManifest {
        source_url: metadata.url.clone(),
        video_id: metadata.id.clone(),
        title: metadata.title.clone(),
        ..BundleManifest::default()
    };
    manifest.files.insert("video".into(), "video.mp4".into());
    manifest
        .files
        .insert("chapters".into(), "chapters.json".into());
    if transcript_path.is_some() {
        manifest
            .files
            .insert("transcript".into(), "transcript.txt".into());
    }
    if let Some(path) = &thumbnail_path {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            manifest.files.insert("thumbnail".into(), name.into());
        }
    }
    save_manifest(&bundle_dir, &manifest)?;

    Ok(BundleInfo {
        dir: bundle_dir,
        title: metadata.title,
        source_url: metadata.url,
        video_id: metadata.id,
        video_path: Some(video_path),
        transcript_path,
        thumbnail_path,
        chapters: metadata.chapters,
        manifest,
    })
}

fn find_first_extension(dir: &Path, extensions: &[&str]) -> Result<Option<PathBuf>> {
    let entries = fs::read_dir(dir).map_err(|e| AppError::io(dir, e))?;
    for entry in entries {
        let entry = entry.map_err(|e| AppError::io(dir, e))?;
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if extensions
            .iter()
            .any(|wanted| ext.eq_ignore_ascii_case(wanted))
        {
            return Ok(Some(path));
        }
    }
    Ok(None)
}
