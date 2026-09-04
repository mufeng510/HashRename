use std::path::Path;

use crate::core::errors::{HashRenameError, Result};
use crate::core::models::FileEntry;

/// Scan a directory for regular files (non-recursive, no symlinks, no subdirectories).
pub fn scan_directory(dir: &Path) -> Result<Vec<FileEntry>> {
    if !dir.exists() {
        return Err(HashRenameError::NotFound {
            path: dir.to_path_buf(),
        });
    }
    if !dir.is_dir() {
        return Err(HashRenameError::NotADirectory {
            path: dir.to_path_buf(),
        });
    }

    let mut entries = Vec::new();

    let read_dir = std::fs::read_dir(dir).map_err(|e| HashRenameError::Io {
        path: dir.to_path_buf(),
        source: e,
    })?;

    for entry in read_dir {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();

        // Skip symlinks
        if path.is_symlink() {
            continue;
        }

        // Skip non-files (directories, special files)
        if !path.is_file() {
            continue;
        }

        let metadata = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        // Only regular files
        if !metadata.is_file() {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy().to_string();
        let extension = path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();

        entries.push(FileEntry {
            path,
            file_name,
            extension,
            size: metadata.len(),
            md5: None,
            is_duplicate: false,
            is_kept: false,
        });
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_scan_empty_directory() {
        let dir = TempDir::new().unwrap();
        let entries = scan_directory(dir.path()).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_scan_files_only() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("file1.txt"), "hello").unwrap();
        fs::write(dir.path().join("file2.jpg"), "world").unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();
        fs::write(dir.path().join("subdir/file3.txt"), "nested").unwrap();

        let entries = scan_directory(dir.path()).unwrap();
        assert_eq!(entries.len(), 2);
        let names: Vec<&str> = entries.iter().map(|e| e.file_name.as_str()).collect();
        assert!(names.contains(&"file1.txt"));
        assert!(names.contains(&"file2.jpg"));
    }

    #[test]
    fn test_scan_not_a_directory() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("not_a_dir.txt");
        fs::write(&file, "data").unwrap();
        let result = scan_directory(&file);
        assert!(result.is_err());
    }

    #[test]
    fn test_scan_nonexistent_directory() {
        let result = scan_directory(Path::new("/nonexistent/path/that/does/not/exist"));
        assert!(result.is_err());
    }

    #[test]
    fn test_scan_preserves_extension_case() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Photo.JPG"), "data").unwrap();
        fs::write(dir.path().join("image.webp"), "data").unwrap();

        let entries = scan_directory(dir.path()).unwrap();
        let exts: Vec<&str> = entries.iter().map(|e| e.extension.as_str()).collect();
        assert!(exts.contains(&"JPG"));
        assert!(exts.contains(&"webp"));
    }
}
