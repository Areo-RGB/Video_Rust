use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("XML error: {0}")]
    Xml(#[from] quick_xml::DeError),
    #[error("process `{program}` failed with exit code {code:?}: {stderr}")]
    Process {
        program: String,
        code: Option<i32>,
        stderr: String,
    },
    #[error("missing required value: {0}")]
    Missing(String),
    #[error("invalid data: {0}")]
    Invalid(String),
    #[error("{0}")]
    Message(String),
}

impl AppError {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
