use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HashRenameError {
    #[error("IO error for {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Access denied: {path}")]
    AccessDenied { path: PathBuf },

    #[error("File not found: {path}")]
    NotFound { path: PathBuf },

    #[error("Not a directory: {path}")]
    NotADirectory { path: PathBuf },

    #[error("Empty directory: {path}")]
    EmptyDirectory { path: PathBuf },

    #[error("Trash operation failed for {path}: {reason}")]
    TrashFailed { path: PathBuf, reason: String },

    #[error("Rename conflict: {path}")]
    RenameConflict { path: PathBuf },

    #[error("Processing incomplete: {0}")]
    ProcessingIncomplete(String),
}

pub type Result<T> = std::result::Result<T, HashRenameError>;
