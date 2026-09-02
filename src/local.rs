use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{AppError, Result};
use crate::model::{BundleInfo, BundleManifest, Chapter, ChapterFile};
use crate::process::run_checked;
use crate::youtube::sanitize_filename;

pub fn scan_bundles(workspace: &Path) -> Result<Vec<PathBuf>> {
    if !workspace.exists() {
        return Ok(Vec::new());
    }
    let mut roots = vec![workspace.to_path_buf()];
    let legacy = workspace.join("Youtube_Videos");
    if legacy.is_dir() {
        roots.push(legacy);
    }

    let mut result = Vec::new();
    for root in roots {
        let entries = fs::read_dir(&root).map_err(|e| AppError::io(&root, e))?;
        for entry in entries {
            let entry = entry.map_err(|e| AppError::io(&root, e))?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if path.join("chapters.json").is_file() || contains_video(&path)? {
                result.push(path);
            }
        }
    }
    result.sort();
    result.dedup();
    Ok(result)
}

pub fn load_bundle(dir: &Path) -> Result<BundleInfo> {
    let chapter_path = dir.join("chapters.json");
    let chapter_file = if chapter_path.is_file() {
        let raw = fs::read_to_string(&chapter_path).map_err(|e| AppError::io(&chapter_path, e))?;
        serde_json::from_str::<ChapterFile>(&raw)?
    } else {
        ChapterFile {
            video_id: String::new(),
            video_title: dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Local video")
                .to_owned(),
            url: String::new(),
            chapters: Vec::new(),
            available: false,
        }
    };
    let video_path = largest_video(dir)?;
    let transcript_path = optional_file(dir, "transcript.txt");
    let thumbnail_path = [
        "thumbnail.jpg",
        "thumbnail.jpeg",
        "thumbnail.png",
        "thumbnail.webp",
    ]
    .iter()
    .map(|name| dir.join(name))
    .find(|path| path.is_file());
    let manifest = load_manifest(dir).unwrap_or_else(|_| BundleManifest {
        source_url: chapter_file.url.clone(),
        video_id: chapter_file.video_id.clone(),
        title: chapter_file.video_title.clone(),
        files: BTreeMap::new(),
        uploads: BTreeMap::new(),
    });

    Ok(BundleInfo {
        dir: dir.to_path_buf(),
        title: if chapter_file.video_title.is_empty() {
            dir.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Local video")
                .to_owned()
        } else {
            chapter_file.video_title
        },
        source_url: chapter_file.url,
        video_id: chapter_file.video_id,
        video_path,
        transcript_path,
        thumbnail_path,
        chapters: chapter_file.chapters,
        manifest,
    })
}

pub fn save_chapter_file(dir: &Path, chapter_file: &ChapterFile) -> Result<()> {
    fs::create_dir_all(dir).map_err(|e| AppError::io(dir, e))?;
    let path = dir.join("chapters.json");
    let json = serde_json::to_string_pretty(chapter_file)?;
    fs::write(&path, format!("{json}\n")).map_err(|e| AppError::io(&path, e))
}

pub fn load_manifest(dir: &Path) -> Result<BundleManifest> {
    let path = dir.join("manifest.json");
    if !path.is_file() {
        return Ok(BundleManifest::default());
    }
    let raw = fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))?;
    Ok(serde_json::from_str(&raw)?)
}

pub fn save_manifest(dir: &Path, manifest: &BundleManifest) -> Result<()> {
    fs::create_dir_all(dir).map_err(|e| AppError::io(dir, e))?;
    let path = dir.join("manifest.json");
    let json = serde_json::to_string_pretty(manifest)?;
    fs::write(&path, format!("{json}\n")).map_err(|e| AppError::io(&path, e))
}

pub fn record_uploaded_url(dir: &Path, relative_name: &str, url: &str) -> Result<BundleManifest> {
    let mut manifest = load_manifest(dir)?;
    manifest
        .uploads
        .insert(relative_name.replace('\\', "/"), url.to_owned());
    save_manifest(dir, &manifest)?;
    Ok(manifest)
}

pub fn cut_args(
    ffmpeg: &str,
    input: &Path,
    chapter: &Chapter,
    output: &Path,
) -> (String, Vec<String>) {
    let duration = chapter.duration();
    let args = vec![
        "-y".into(),
        "-ss".into(),
        format!("{:.3}", chapter.start_seconds),
        "-i".into(),
        input.to_string_lossy().into_owned(),
        "-t".into(),
        format!("{duration:.3}"),
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        "veryfast".into(),
        "-crf".into(),
        "20".into(),
        "-c:a".into(),
        "aac".into(),
        "-movflags".into(),
        "+faststart".into(),
        output.to_string_lossy().into_owned(),
    ];
    (ffmpeg.to_owned(), args)
}

pub fn cut_relative_name(chapter_index: usize, chapter_name: &str) -> String {
    format!(
        "cuts/{:02}-{}.mp4",
        chapter_index + 1,
        sanitize_filename(chapter_name)
    )
}

pub fn bundle_object_name(bundle_title: &str, relative_name: &str) -> String {
    let folder = sanitize_filename(bundle_title).replace(' ', "_");
    let relative = relative_name
        .trim_start_matches('/')
        .trim_start_matches("cuts/");
    format!("{folder}/{relative}")
}

pub fn cut_chapter(ffmpeg: &str, bundle: &BundleInfo, chapter_index: usize) -> Result<PathBuf> {
    let video = bundle
        .video_path
        .as_ref()
        .ok_or_else(|| AppError::Missing("bundle has no MP4 video".into()))?;
    let chapter = bundle
        .chapters
        .get(chapter_index)
        .ok_or_else(|| AppError::Invalid("chapter index out of range".into()))?;
    let cuts = bundle.dir.join("cuts");
    fs::create_dir_all(&cuts).map_err(|e| AppError::io(&cuts, e))?;
    let relative = cut_relative_name(chapter_index, &chapter.name);
    let output = bundle.dir.join(&relative);
    let (program, args) = cut_args(ffmpeg, video, chapter, &output);
    run_checked(&program, &args)?;
    Ok(output)
}

pub fn cut_all(ffmpeg: &str, bundle: &BundleInfo) -> Result<Vec<PathBuf>> {
    (0..bundle.chapters.len())
        .map(|index| cut_chapter(ffmpeg, bundle, index))
        .collect()
}

fn optional_file(dir: &Path, name: &str) -> Option<PathBuf> {
    let path = dir.join(name);
    path.is_file().then_some(path)
}

fn contains_video(dir: &Path) -> Result<bool> {
    Ok(largest_video(dir)?.is_some())
}

fn largest_video(dir: &Path) -> Result<Option<PathBuf>> {
    let mut candidates = Vec::new();
    let entries = fs::read_dir(dir).map_err(|e| AppError::io(dir, e))?;
    for entry in entries {
        let entry = entry.map_err(|e| AppError::io(dir, e))?;
        let path = entry.path();
        let is_video = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| {
                matches!(
                    ext.to_ascii_lowercase().as_str(),
                    "mp4" | "mkv" | "webm" | "mov"
                )
            });
        if is_video && path.is_file() {
            let len = path.metadata().map_err(|e| AppError::io(&path, e))?.len();
            candidates.push((len, path));
        }
    }
    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    Ok(candidates.into_iter().next().map(|(_, path)| path))
}
