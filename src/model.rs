use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Chapter {
    pub name: String,
    #[serde(rename = "startSeconds")]
    pub start_seconds: f64,
    #[serde(rename = "endSeconds")]
    pub end_seconds: f64,
    #[serde(rename = "videoUrl", default, skip_serializing_if = "String::is_empty")]
    pub video_url: String,
    #[serde(
        rename = "thumbnailUrl",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub thumbnail_url: String,
}

impl Chapter {
    pub fn duration(&self) -> f64 {
        (self.end_seconds - self.start_seconds).max(0.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChapterFile {
    #[serde(rename = "videoId", default)]
    pub video_id: String,
    #[serde(rename = "videoTitle", default)]
    pub video_title: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub chapters: Vec<Chapter>,
    #[serde(default)]
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoMetadata {
    pub id: String,
    pub title: String,
    pub url: String,
    #[serde(default)]
    pub chapters: Vec<Chapter>,
    #[serde(default)]
    pub duration: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct BundleManifest {
    #[serde(default)]
    pub source_url: String,
    #[serde(default)]
    pub video_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    #[serde(default)]
    pub uploads: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BundleInfo {
    pub dir: PathBuf,
    pub title: String,
    pub source_url: String,
    pub video_id: String,
    pub video_path: Option<PathBuf>,
    pub transcript_path: Option<PathBuf>,
    pub thumbnail_path: Option<PathBuf>,
    pub chapters: Vec<Chapter>,
    pub manifest: BundleManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Playlist {
    pub name: String,
    #[serde(rename = "type", default, skip_serializing_if = "String::is_empty")]
    pub kind: String,
    #[serde(rename = "videoId", default, skip_serializing_if = "String::is_empty")]
    pub video_id: String,
    #[serde(rename = "videoUrl", default, skip_serializing_if = "String::is_empty")]
    pub video_url: String,
    #[serde(
        rename = "thumbnailUrl",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub thumbnail_url: String,
    #[serde(default)]
    pub chapters: Vec<Chapter>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DataFile {
    #[serde(default)]
    pub playlists: Vec<Playlist>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct R2Object {
    pub key: String,
    pub size: u64,
    pub last_modified: String,
    pub public_url: String,
}
