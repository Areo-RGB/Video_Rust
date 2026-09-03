use std::fs;
use std::path::Path;

use crate::error::{AppError, Result};
use crate::local::cut_relative_name;
use crate::model::{BundleInfo, Chapter, DataFile, Playlist};

pub fn parse_data(text: &str) -> Result<DataFile> {
    let value: serde_json::Value = serde_json::from_str(text)?;
    if value.is_array() {
        let playlists: Vec<Playlist> = serde_json::from_value(value)?;
        Ok(DataFile { playlists })
    } else {
        Ok(serde_json::from_value(value)?)
    }
}

pub fn load_data(path: &Path) -> Result<DataFile> {
    let text = fs::read_to_string(path).map_err(|e| AppError::io(path, e))?;
    parse_data(&text)
}

pub fn fetch_data(url: &str) -> Result<DataFile> {
    let text = reqwest::blocking::Client::new()
        .get(url)
        .send()?
        .error_for_status()?
        .text()?;
    parse_data(&text)
}

pub fn save_data(path: &Path, data: &DataFile) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }
    let text = serde_json::to_string_pretty(&data.playlists)?;
    fs::write(path, format!("{text}\n")).map_err(|e| AppError::io(path, e))
}

pub fn add_playlist(data: &mut DataFile, name: &str) -> Result<usize> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::Invalid("playlist name cannot be empty".into()));
    }
    if data
        .playlists
        .iter()
        .any(|p| p.name.eq_ignore_ascii_case(name))
    {
        return Err(AppError::Invalid(format!(
            "playlist `{name}` already exists"
        )));
    }
    data.playlists.push(Playlist {
        name: name.to_owned(),
        kind: "direct".into(),
        ..Playlist::default()
    });
    Ok(data.playlists.len() - 1)
}

pub fn remove_playlist(data: &mut DataFile, playlist_index: usize) -> Result<Playlist> {
    if playlist_index >= data.playlists.len() {
        return Err(AppError::Invalid("playlist index out of range".into()));
    }
    Ok(data.playlists.remove(playlist_index))
}

pub fn add_chapter(data: &mut DataFile, playlist_index: usize, chapter: Chapter) -> Result<usize> {
    let playlist = data
        .playlists
        .get_mut(playlist_index)
        .ok_or_else(|| AppError::Invalid("playlist index out of range".into()))?;
    if chapter.name.trim().is_empty() {
        return Err(AppError::Invalid("chapter name cannot be empty".into()));
    }
    if chapter.end_seconds < chapter.start_seconds {
        return Err(AppError::Invalid(
            "chapter end must be greater than or equal to start".into(),
        ));
    }
    playlist.chapters.push(chapter);
    Ok(playlist.chapters.len() - 1)
}

pub fn remove_chapter(
    data: &mut DataFile,
    playlist_index: usize,
    chapter_index: usize,
) -> Result<Chapter> {
    let playlist = data
        .playlists
        .get_mut(playlist_index)
        .ok_or_else(|| AppError::Invalid("playlist index out of range".into()))?;
    if chapter_index >= playlist.chapters.len() {
        return Err(AppError::Invalid("chapter index out of range".into()));
    }
    Ok(playlist.chapters.remove(chapter_index))
}

pub fn move_chapter(
    data: &mut DataFile,
    playlist_index: usize,
    from: usize,
    to: usize,
) -> Result<()> {
    let playlist = data
        .playlists
        .get_mut(playlist_index)
        .ok_or_else(|| AppError::Invalid("playlist index out of range".into()))?;
    if from >= playlist.chapters.len() || to >= playlist.chapters.len() {
        return Err(AppError::Invalid("chapter index out of range".into()));
    }
    if from == to {
        return Ok(());
    }
    let chapter = playlist.chapters.remove(from);
    playlist.chapters.insert(to, chapter);
    Ok(())
}

pub fn playlist_from_bundle(bundle: &BundleInfo) -> Playlist {
    let full_video_url = bundle
        .video_path
        .as_ref()
        .and_then(|path| relative_upload(bundle, path))
        .unwrap_or_default();
    let thumbnail_url = bundle
        .thumbnail_path
        .as_ref()
        .and_then(|path| relative_upload(bundle, path))
        .unwrap_or_default();

    let chapters = bundle
        .chapters
        .iter()
        .enumerate()
        .map(|(index, chapter)| {
            let mut chapter = chapter.clone();
            let cut_url = bundle
                .manifest
                .uploads
                .get(&cut_relative_name(index, &chapter.name))
                .cloned()
                .unwrap_or_default();
            chapter.video_url = if cut_url.is_empty() {
                full_video_url.clone()
            } else {
                cut_url
            };
            if chapter.thumbnail_url.is_empty() {
                chapter.thumbnail_url = thumbnail_url.clone();
            }
            chapter
        })
        .collect();

    Playlist {
        name: bundle.title.clone(),
        kind: "direct".into(),
        video_id: bundle.video_id.clone(),
        video_url: full_video_url,
        thumbnail_url,
        chapters,
    }
}

pub fn upsert_bundle_playlist(data: &mut DataFile, bundle: &BundleInfo) -> usize {
    let playlist = playlist_from_bundle(bundle);
    let existing = data.playlists.iter().position(|candidate| {
        (!playlist.video_id.is_empty() && candidate.video_id == playlist.video_id)
            || candidate.name.eq_ignore_ascii_case(&playlist.name)
    });
    if let Some(index) = existing {
        data.playlists[index] = playlist;
        index
    } else {
        data.playlists.push(playlist);
        data.playlists.len() - 1
    }
}

fn relative_upload(bundle: &BundleInfo, path: &Path) -> Option<String> {
    let relative = path
        .strip_prefix(&bundle.dir)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    bundle.manifest.uploads.get(&relative).cloned()
}
