use std::path::PathBuf;
use std::sync::mpsc::Sender;

use crate::error::Result;
use crate::model::{BundleInfo, R2Object, VideoMetadata};

#[derive(Debug)]
pub enum JobValue {
    Unit,
    Metadata(VideoMetadata),
    Bundle(BundleInfo),
    Bundles(Vec<PathBuf>),
    Path(PathBuf),
    Paths(Vec<PathBuf>),
    R2Objects(Vec<R2Object>),
    Uploaded {
        bundle_dir: Option<PathBuf>,
        relative_name: String,
        url: String,
    },
}

#[derive(Debug)]
pub struct JobMessage {
    pub label: String,
    pub result: std::result::Result<JobValue, String>,
}

pub fn spawn_job<F>(sender: Sender<JobMessage>, label: impl Into<String>, job: F)
where
    F: FnOnce() -> Result<JobValue> + Send + 'static,
{
    let label = label.into();
    std::thread::spawn(move || {
        let result = job().map_err(|error| error.to_string());
        let _ = sender.send(JobMessage { label, result });
    });
}
