use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::core::errors::{HashRenameError, Result};
use crate::core::hasher::{Hasher, Md5Hasher};
use crate::core::models::{DuplicateGroup, FileEntry};
use crate::core::sorter;

/// Detect duplicate files using size pre-filtering, MD5 hashing, and content verification.
/// Returns (kept_files, duplicate_groups).
pub fn detect_duplicates(
    mut files: Vec<FileEntry>,
) -> Result<(Vec<FileEntry>, Vec<DuplicateGroup>)> {
    let hasher = Md5Hasher::new();

    // Step 1: Group by file size
    let mut size_groups: HashMap<u64, Vec<usize>> = HashMap::new();
    for (i, file) in files.iter().enumerate() {
        size_groups.entry(file.size).or_default().push(i);
    }

    // Step 2: For groups with 2+ files, compute MD5
    let mut md5_groups: HashMap<String, Vec<usize>> = HashMap::new();

    for (_size, indices) in &size_groups {
        if indices.len() < 2 {
            continue;
        }
        for &idx in indices {
            let hash = hasher.hash(&files[idx].path).map_err(|e| {
                log::warn!("Failed to hash {}: {}", files[idx].path.display(), e);
                e
            })?;
            files[idx].md5 = Some(hash.clone());
            md5_groups.entry(hash).or_default().push(idx);
        }
    }

    // Step 3: For MD5-matched groups, do byte-by-byte content verification
    let mut duplicate_groups = Vec::new();

    for (md5_hash, indices) in &md5_groups {
        if indices.len() < 2 {
            continue;
        }

        let size = files[indices[0]].size;

        // Verify all files in this MD5 group have identical content
        let verified = verify_group_content(&files, indices)?;

        if verified.len() < 2 {
            continue;
        }

        // Step 4: Select keeper by natural sort (first in natural sort order)
        let mut verified_entries: Vec<FileEntry> = verified
            .iter()
            .map(|&idx| files[idx].clone())
            .collect();

        sorter::natural_sort_files(&mut verified_entries);

        let kept_file = verified_entries[0].path.clone();
        let duplicate_files: Vec<std::path::PathBuf> =
            verified_entries[1..].iter().map(|e| e.path.clone()).collect();

        // Mark files
        for &idx in &verified {
            if files[idx].path == kept_file {
                files[idx].is_kept = true;
                files[idx].is_duplicate = false;
            } else {
                files[idx].is_duplicate = true;
                files[idx].is_kept = false;
            }
        }

        duplicate_groups.push(DuplicateGroup {
            md5: md5_hash.clone(),
            size,
            files: verified_entries,
            kept_file,
            duplicate_files,
        });
    }

    let kept_files: Vec<FileEntry> = files.into_iter().filter(|f| !f.is_duplicate).collect();

    Ok((kept_files, duplicate_groups))
}

/// Verify that all files in a group have identical content byte-by-byte.
fn verify_group_content(files: &[FileEntry], indices: &[usize]) -> Result<Vec<usize>> {
    if indices.is_empty() {
        return Ok(vec![]);
    }

    // Read first file as reference
    let reference_content = read_file_bytes(&files[indices[0]].path)?;
    let mut verified = vec![indices[0]];

    for &idx in &indices[1..] {
        let content = read_file_bytes(&files[idx].path)?;
        if content == reference_content {
            verified.push(idx);
        } else {
            log::warn!(
                "MD5 collision detected: {} has same hash but different content from {}",
                files[idx].path.display(),
                files[indices[0]].path.display()
            );
        }
    }

    Ok(verified)
}

/// Read entire file into memory for content comparison.
/// Only used for small files in verification step.
fn read_file_bytes(path: &Path) -> Result<Vec<u8>> {
    let mut file = File::open(path).map_err(|e| HashRenameError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    let mut content = Vec::new();
    file.read_to_end(&mut content).map_err(|e| HashRenameError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str, content: &[u8]) -> std::path::PathBuf {
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_no_duplicates() {
        let dir = TempDir::new().unwrap();
        let files = vec![
            FileEntry {
                path: create_test_file(dir.path(), "a.txt", b"hello"),
                file_name: "a.txt".to_string(),
                extension: "txt".to_string(),
                size: 5,
                md5: None,
                is_duplicate: false,
                is_kept: false,
            },
            FileEntry {
                path: create_test_file(dir.path(), "b.txt", b"world"),
                file_name: "b.txt".to_string(),
                extension: "txt".to_string(),
                size: 5,
                md5: None,
                is_duplicate: false,
                is_kept: false,
            },
        ];

        let (kept, groups) = detect_duplicates(files).unwrap();
        assert_eq!(kept.len(), 2);
        assert!(groups.is_empty());
    }

    #[test]
    fn test_identical_files() {
        let dir = TempDir::new().unwrap();
        let content = b"identical content here";
        let files = vec![
            FileEntry {
                path: create_test_file(dir.path(), "a.txt", content),
                file_name: "a.txt".to_string(),
                extension: "txt".to_string(),
                size: content.len() as u64,
                md5: None,
                is_duplicate: false,
                is_kept: false,
            },
            FileEntry {
                path: create_test_file(dir.path(), "b.txt", content),
                file_name: "b.txt".to_string(),
                extension: "txt".to_string(),
                size: content.len() as u64,
                md5: None,
                is_duplicate: false,
                is_kept: false,
            },
        ];

        let (kept, groups) = detect_duplicates(files).unwrap();
        assert_eq!(kept.len(), 1);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].duplicate_files.len(), 1);
    }

    #[test]
    fn test_same_size_different_content() {
        let dir = TempDir::new().unwrap();
        let files = vec![
            FileEntry {
                path: create_test_file(dir.path(), "a.txt", b"hello"),
                file_name: "a.txt".to_string(),
                extension: "txt".to_string(),
                size: 5,
                md5: None,
                is_duplicate: false,
                is_kept: false,
            },
            FileEntry {
                path: create_test_file(dir.path(), "b.txt", b"world"),
                file_name: "b.txt".to_string(),
                extension: "txt".to_string(),
                size: 5,
                md5: None,
                is_duplicate: false,
                is_kept: false,
            },
        ];

        let (kept, groups) = detect_duplicates(files).unwrap();
        assert_eq!(kept.len(), 2);
        assert!(groups.is_empty());
    }

    #[test]
    fn test_keeper_selection_natural_sort() {
        let dir = TempDir::new().unwrap();
        let content = b"test content";
        let files = vec![
            FileEntry {
                path: create_test_file(dir.path(), "IMG_10.jpg", content),
                file_name: "IMG_10.jpg".to_string(),
                extension: "jpg".to_string(),
                size: content.len() as u64,
                md5: None,
                is_duplicate: false,
                is_kept: false,
            },
            FileEntry {
                path: create_test_file(dir.path(), "IMG_2.jpg", content),
                file_name: "IMG_2.jpg".to_string(),
                extension: "jpg".to_string(),
                size: content.len() as u64,
                md5: None,
                is_duplicate: false,
                is_kept: false,
            },
            FileEntry {
                path: create_test_file(dir.path(), "IMG_1.jpg", content),
                file_name: "IMG_1.jpg".to_string(),
                extension: "jpg".to_string(),
                size: content.len() as u64,
                md5: None,
                is_duplicate: false,
                is_kept: false,
            },
        ];

        let (_kept, groups) = detect_duplicates(files).unwrap();
        assert_eq!(groups.len(), 1);
        let group = &groups[0];
        // IMG_1.jpg should be kept (first in natural sort)
        assert!(group.kept_file.to_string_lossy().contains("IMG_1"));
        assert_eq!(group.duplicate_files.len(), 2);
    }
}
