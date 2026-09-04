use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: PathBuf,
    pub file_name: String,
    pub extension: String,
    pub size: u64,
    pub md5: Option<String>,
    pub is_duplicate: bool,
    pub is_kept: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub md5: String,
    pub size: u64,
    pub files: Vec<FileEntry>,
    pub kept_file: PathBuf,
    pub duplicate_files: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameOperation {
    pub old_path: PathBuf,
    pub temporary_path: PathBuf,
    pub new_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingResult {
    pub scanned_count: usize,
    pub duplicate_count: usize,
    pub trashed_count: usize,
    pub renamed_count: usize,
    pub failed_count: usize,
    pub elapsed_secs: f64,
    pub errors: Vec<ProcessingError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingError {
    pub path: PathBuf,
    pub operation: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressInfo {
    pub stage: String,
    pub scanned_count: usize,
    pub duplicate_count: usize,
    pub kept_count: usize,
    pub current_file: String,
    pub progress_percent: f32,
}
