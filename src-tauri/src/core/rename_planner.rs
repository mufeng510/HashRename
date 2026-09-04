use std::collections::HashSet;
use std::path::Path;

use crate::core::errors::{HashRenameError, Result};
use crate::core::models::{FileEntry, RenameOperation};
use crate::core::sorter;

/// Plan rename operations for kept files.
/// Returns list of RenameOperation (old_path -> temporary_path -> new_path).
pub fn plan_renames(
    mut files: Vec<FileEntry>,
    directory: &Path,
) -> Result<Vec<RenameOperation>> {
    if files.is_empty() {
        return Ok(Vec::new());
    }

    // Sort files by natural order of filename
    sorter::natural_sort_files(&mut files);

    // Determine digit width
    let count = files.len();
    let digit_width = std::cmp::max(3, count.to_string().len());

    // Collect existing filenames in the directory to avoid conflicts
    let existing_names: HashSet<String> = std::fs::read_dir(directory)
        .map_err(|e| HashRenameError::Io {
            path: directory.to_path_buf(),
            source: e,
        })?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if e.path().is_file() {
                Some(name)
            } else {
                None
            }
        })
        .collect();

    let mut operations = Vec::new();
    let mut new_names_used: HashSet<String> = HashSet::new();

    for (i, file) in files.iter().enumerate() {
        let number = i + 1;
        let padded_number = format!("{:0width$}", number, width = digit_width);
        let new_name = format!("{}.{}", padded_number, file.extension);

        // Skip if already correctly named
        if file.file_name == new_name {
            continue;
        }

        // Check if new name conflicts with existing files or planned names
        if existing_names.contains(&new_name) && !new_names_used.contains(&new_name) {
            // This target name exists but isn't part of our rename plan
            // We need to use a different approach - rename to temp first
            log::warn!(
                "Target name {} already exists in directory, using temp rename",
                new_name
            );
        }

        if new_names_used.contains(&new_name) {
            return Err(HashRenameError::RenameConflict {
                path: directory.join(&new_name),
            });
        }

        // Generate unique temporary name
        let temp_name = generate_temp_name(i, directory)?;
        let temp_path = directory.join(&temp_name);
        let new_path = directory.join(&new_name);

        operations.push(RenameOperation {
            old_path: file.path.clone(),
            temporary_path: temp_path,
            new_path: new_path.clone(),
        });

        new_names_used.insert(new_name);
    }

    Ok(operations)
}

/// Generate a unique temporary filename that doesn't exist in the directory.
fn generate_temp_name(index: usize, directory: &Path) -> Result<String> {
    // Use a combination of PID, timestamp, and index for uniqueness
    let pid = std::process::id();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    for attempt in 0..1000 {
        let hash_input = format!("{}-{}-{}-{}", pid, now, index, attempt);
        let hash = simple_hash(&hash_input);
        let name = format!(".hashrename_tmp_{:016x}", hash);

        let path = directory.join(&name);
        if !path.exists() {
            return Ok(name);
        }
    }

    Err(HashRenameError::RenameConflict {
        path: directory.to_path_buf(),
    })
}

/// Simple non-cryptographic hash for generating unique temp names.
fn simple_hash(input: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325; // FNV offset basis
    for byte in input.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3); // FNV prime
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str) -> FileEntry {
        let path = dir.join(name);
        fs::write(&path, "test").unwrap();
        FileEntry {
            path,
            file_name: name.to_string(),
            extension: name
                .rsplit('.')
                .next()
                .unwrap_or("")
                .to_string(),
            size: 4,
            md5: None,
            is_duplicate: false,
            is_kept: false,
        }
    }

    #[test]
    fn test_plan_renames_basic() {
        let dir = TempDir::new().unwrap();
        let files = vec![
            create_test_file(dir.path(), "c.txt"),
            create_test_file(dir.path(), "a.txt"),
            create_test_file(dir.path(), "b.txt"),
        ];

        let ops = plan_renames(files, dir.path()).unwrap();
        assert_eq!(ops.len(), 3);

        // Should be sorted: a.txt -> 001.txt, b.txt -> 002.txt, c.txt -> 003.txt
        assert!(ops[0].new_path.to_string_lossy().contains("001"));
        assert!(ops[1].new_path.to_string_lossy().contains("002"));
        assert!(ops[2].new_path.to_string_lossy().contains("003"));
    }

    #[test]
    fn test_plan_renames_empty() {
        let dir = TempDir::new().unwrap();
        let ops = plan_renames(vec![], dir.path()).unwrap();
        assert!(ops.is_empty());
    }

    #[test]
    fn test_plan_renames_already_named() {
        let dir = TempDir::new().unwrap();
        // Create files that would already have the target names
        fs::write(dir.path().join("001.txt"), "test1").unwrap();
        fs::write(dir.path().join("002.txt"), "test2").unwrap();

        let files = vec![
            FileEntry {
                path: dir.path().join("001.txt"),
                file_name: "001.txt".to_string(),
                extension: "txt".to_string(),
                size: 5,
                md5: None,
                is_duplicate: false,
                is_kept: false,
            },
            FileEntry {
                path: dir.path().join("002.txt"),
                file_name: "002.txt".to_string(),
                extension: "txt".to_string(),
                size: 5,
                md5: None,
                is_duplicate: false,
                is_kept: false,
            },
        ];

        let ops = plan_renames(files, dir.path()).unwrap();
        // Files already correctly named should be skipped
        assert!(ops.is_empty());
    }

    #[test]
    fn test_plan_renames_digit_width() {
        let dir = TempDir::new().unwrap();
        let mut files = Vec::new();
        for i in 0..1001 {
            files.push(create_test_file(dir.path(), &format!("file_{}.txt", i)));
        }

        let ops = plan_renames(files, dir.path()).unwrap();
        assert_eq!(ops.len(), 1001);

        // Check that digit width is 4 for > 999 files
        let first_name = ops[0].new_path.file_name().unwrap().to_string_lossy();
        assert!(first_name.starts_with("0001"));
    }

    #[test]
    fn test_plan_renames_preserves_extension_case() {
        let dir = TempDir::new().unwrap();
        let files = vec![
            create_test_file(dir.path(), "photo.JPG"),
            create_test_file(dir.path(), "image.webp"),
        ];

        let ops = plan_renames(files, dir.path()).unwrap();
        assert_eq!(ops.len(), 2);

        let exts: Vec<&str> = ops
            .iter()
            .map(|op| {
                op.new_path
                    .extension()
                    .unwrap()
                    .to_str()
                    .unwrap()
            })
            .collect();
        assert!(exts.contains(&"JPG"));
        assert!(exts.contains(&"webp"));
    }

    #[test]
    fn test_simple_hash_deterministic() {
        assert_eq!(simple_hash("test"), simple_hash("test"));
        assert_ne!(simple_hash("test"), simple_hash("other"));
    }
}
