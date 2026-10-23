use std::path::{Path, PathBuf};
use thiserror::Error;

/// A problem in a pack, always pointing at the file (and line) to fix.
#[derive(Debug, Error)]
pub enum PackError {
    #[error("{}: cannot read: {source}", path.display())]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{}:{line}:{column}: {message}", path.display())]
    Syntax {
        path: PathBuf,
        line: usize,
        column: usize,
        message: String,
    },
    #[error("{}: {message}", path.display())]
    Invalid { path: PathBuf, message: String },
}

impl PackError {
    pub fn invalid(path: &Path, message: impl Into<String>) -> Self {
        Self::Invalid {
            path: path.to_owned(),
            message: message.into(),
        }
    }

    pub fn io(path: &Path, source: std::io::Error) -> Self {
        Self::Io {
            path: path.to_owned(),
            source,
        }
    }

    pub fn syntax(path: &Path, error: &ron::error::SpannedError) -> Self {
        Self::Syntax {
            path: path.to_owned(),
            line: error.span.start.line,
            column: error.span.start.col,
            message: error.code.to_string(),
        }
    }
}
