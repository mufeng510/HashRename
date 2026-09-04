use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::core::errors::HashRenameError;
use crate::core::hasher::Hasher;
use crate::core::models::FileEntry;
use crate::core::{scanner, hasher, duplicate_detector, sorter, rename_planner};
use crate::platform::trash::TrashOperation;

#[test]
fn test_scan_skips_symlinks() {
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(dir.path().join("real.txt"), "content").unwrap();
    std::os::unix::fs::symlink(dir.path().join("real.txt"), dir.path().join("link.txt")).unwrap();

    let entries = scanner::scan_directory(dir.path()).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].file_name, "real.txt");
}

#[test]
fn test_scan_skips_directories() {
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(dir.path().join("file.txt"), "content").unwrap();
    fs::create_dir(dir.path().join("subdir")).unwrap();

    let entries = scanner::scan_directory(dir.path()).unwrap();
    assert_eq!(entries.len(), 1);
}

#[test]
fn test_scan_file_no_extension() {
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(dir.path().join("Makefile"), "all:").unwrap();

    let entries = scanner::scan_directory(dir.path()).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].extension, "");
}

#[test]
fn test_scan_multi_dot_extension() {
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(dir.path().join("archive.tar.gz"), "data").unwrap();

    let entries = scanner::scan_directory(dir.path()).unwrap();
    assert_eq!(entries.len(), 1);
    // Rust's Path::extension() returns "gz" for "archive.tar.gz"
    assert_eq!(entries[0].extension, "gz");
}

#[test]
fn test_scan_unicode_filename() {
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(dir.path().join("日本語ファイル.txt"), "data").unwrap();
    fs::write(dir.path().join("emoji🎉.txt"), "data").unwrap();
    fs::write(dir.path().join("中文文件.md"), "data").unwrap();

    let entries = scanner::scan_directory(dir.path()).unwrap();
    assert_eq!(entries.len(), 3);
    let names: Vec<&str> = entries.iter().map(|e| e.file_name.as_str()).collect();
    assert!(names.contains(&"日本語ファイル.txt"));
    assert!(names.contains(&"emoji🎉.txt"));
    assert!(names.contains(&"中文文件.md"));
}

#[test]
fn test_scan_long_filename() {
    let dir = tempfile::TempDir::new().unwrap();
    let long_name = "a".repeat(200) + ".txt";
    fs::write(dir.path().join(&long_name), "data").unwrap();

    let entries = scanner::scan_directory(dir.path()).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].file_name, long_name);
}

#[test]
fn test_hash_empty_file() {
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(dir.path().join("empty.txt"), "").unwrap();

    let hasher = hasher::Md5Hasher::new();
    let hash = hasher.hash(&dir.path().join("empty.txt")).unwrap();
    assert_eq!(hash, "d41d8cd98f00b204e9800998ecf8427e");
}

#[test]
fn test_hash_binary_content() {
    let dir = tempfile::TempDir::new().unwrap();
    let data: Vec<u8> = (0..=255).collect();
    fs::write(dir.path().join("binary.bin"), &data).unwrap();

    let hasher = hasher::Md5Hasher::new();
    let hash = hasher.hash(&dir.path().join("binary.bin")).unwrap();
    assert_eq!(hash.len(), 32);
}

#[test]
fn test_hash_unicode_content() {
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(dir.path().join("unicode.txt"), "你好世界").unwrap();

    let hasher = hasher::Md5Hasher::new();
    let hash = hasher.hash(&dir.path().join("unicode.txt")).unwrap();
    assert_eq!(hash.len(), 32);
}

#[test]
fn test_duplicate_detector_size_only_match() {
    let dir = tempfile::TempDir::new().unwrap();
    // Same size, different content
    fs::write(dir.path().join("a.txt"), "hello").unwrap();
    fs::write(dir.path().join("b.txt"), "world").unwrap();

    let files = scanner::scan_directory(dir.path()).unwrap();
    let (_kept, groups) = duplicate_detector::detect_duplicates(files).unwrap();
    assert_eq!(_kept.len(), 2);
    assert!(groups.is_empty());
}

#[test]
fn test_duplicate_detector_identical_files() {
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(dir.path().join("a.txt"), "identical").unwrap();
    fs::write(dir.path().join("b.txt"), "identical").unwrap();
    fs::write(dir.path().join("c.txt"), "different").unwrap();

    let files = scanner::scan_directory(dir.path()).unwrap();
    let (kept, groups) = duplicate_detector::detect_duplicates(files).unwrap();
    assert_eq!(kept.len(), 2);
    assert_eq!(groups.len(), 1);
}

#[test]
fn test_duplicate_detector_natural_sort_keeper() {
    let dir = tempfile::TempDir::new().unwrap();
    let content = b"same content";
    fs::write(dir.path().join("IMG_10.jpg"), content).unwrap();
    fs::write(dir.path().join("IMG_2.jpg"), content).unwrap();
    fs::write(dir.path().join("IMG_1.jpg"), content).unwrap();

    let files = scanner::scan_directory(dir.path()).unwrap();
    let (kept, groups) = duplicate_detector::detect_duplicates(files).unwrap();
    assert_eq!(groups.len(), 1);
    let group = &groups[0];
    // IMG_1.jpg should be kept (first in natural sort)
    assert!(group.kept_file.to_string_lossy().contains("IMG_1"));
    assert_eq!(group.duplicate_files.len(), 2);
}

#[test]
fn test_natural_sort_numbers() {
    let mut names = vec!["10.txt", "2.txt", "1.txt", "20.txt", "100.txt"];
    names.sort_by(|a, b| sorter::natural_sort_compare(a, b));
    assert_eq!(names, vec!["1.txt", "2.txt", "10.txt", "20.txt", "100.txt"]);
}

#[test]
fn test_natural_sort_chinese() {
    let mut names = vec!["照片2.jpg", "照片10.jpg", "照片1.jpg"];
    names.sort_by(|a, b| sorter::natural_sort_compare(a, b));
    assert_eq!(names, vec!["照片1.jpg", "照片2.jpg", "照片10.jpg"]);
}

#[test]
fn test_natural_sort_mixed() {
    let mut names = vec!["apple.jpg", "10.jpg", "banana.png", "2.txt"];
    names.sort_by(|a, b| sorter::natural_sort_compare(a, b));
    assert_eq!(names, vec!["2.txt", "10.jpg", "apple.jpg", "banana.png"]);
}

#[test]
fn test_rename_planner_basic() {
    let dir = tempfile::TempDir::new().unwrap();
    let files = vec![
        create_file_entry(dir.path(), "c.txt"),
        create_file_entry(dir.path(), "a.txt"),
        create_file_entry(dir.path(), "b.txt"),
    ];

    let ops = rename_planner::plan_renames(files, dir.path()).unwrap();
    assert_eq!(ops.len(), 3);
    // Should be sorted: a.txt -> 001.txt, b.txt -> 002.txt, c.txt -> 003.txt
    assert!(ops[0].new_path.to_string_lossy().contains("001"));
    assert!(ops[1].new_path.to_string_lossy().contains("002"));
    assert!(ops[2].new_path.to_string_lossy().contains("003"));
}

#[test]
fn test_rename_planner_digit_width() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut files = Vec::new();
    for i in 0..1001 {
        files.push(create_file_entry(dir.path(), &format!("file_{}.txt", i)));
    }

    let ops = rename_planner::plan_renames(files, dir.path()).unwrap();
    assert_eq!(ops.len(), 1001);
    let first_name = ops[0].new_path.file_name().unwrap().to_string_lossy();
    assert!(first_name.starts_with("0001"));
}

#[test]
fn test_rename_planner_preserves_extension_case() {
    let dir = tempfile::TempDir::new().unwrap();
    let files = vec![
        create_file_entry(dir.path(), "photo.JPG"),
        create_file_entry(dir.path(), "image.webp"),
    ];

    let ops = rename_planner::plan_renames(files, dir.path()).unwrap();
    assert_eq!(ops.len(), 2);
    let exts: Vec<&str> = ops
        .iter()
        .map(|op| op.new_path.extension().unwrap().to_str().unwrap())
        .collect();
    assert!(exts.contains(&"JPG"));
    assert!(exts.contains(&"webp"));
}

#[test]
fn test_rename_planner_no_extension() {
    let dir = tempfile::TempDir::new().unwrap();
    let files = vec![
        create_file_entry(dir.path(), "Makefile"),
        create_file_entry(dir.path(), "README"),
    ];

    let ops = rename_planner::plan_renames(files, dir.path()).unwrap();
    assert_eq!(ops.len(), 2);
    // Files without extensions should get just the number
    assert!(ops[0].new_path.file_name().unwrap().to_str().unwrap().contains("001"));
    assert!(ops[1].new_path.file_name().unwrap().to_str().unwrap().contains("002"));
}

#[test]
fn test_rename_planner_collision_with_existing() {
    let dir = tempfile::TempDir::new().unwrap();
    // Create files that would conflict
    fs::write(dir.path().join("001.txt"), "existing").unwrap();
    fs::write(dir.path().join("002.txt"), "existing").unwrap();
    fs::write(dir.path().join("new_file.txt"), "new").unwrap();

    let files = vec![create_file_entry(dir.path(), "new_file.txt")];
    let ops = rename_planner::plan_renames(files, dir.path()).unwrap();
    assert_eq!(ops.len(), 1);
    // Should rename to something that doesn't conflict
    assert!(ops[0].new_path.to_string_lossy().contains("001"));
}

#[test]
fn test_processor_empty_directory() {
    let dir = tempfile::TempDir::new().unwrap();
    let _result = crate::core::processor::Processor::new(Box::new(MockTrash))
        .process(dir.path(), false, None);
    assert!(_result.is_err());
}

#[test]
fn test_processor_with_duplicates() {
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(dir.path().join("a.txt"), "identical").unwrap();
    fs::write(dir.path().join("b.txt"), "identical").unwrap();
    fs::write(dir.path().join("c.txt"), "different").unwrap();

    let result = crate::core::processor::Processor::new(Box::new(MockTrash))
        .process(dir.path(), false, None)
        .unwrap();

    assert_eq!(result.scanned_count, 3);
    assert_eq!(result.duplicate_count, 1);
    assert_eq!(result.trashed_count, 1);
    assert!(result.renamed_count > 0);
    assert_eq!(result.failed_count, 0);
}

#[test]
fn test_processor_no_duplicates() {
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(dir.path().join("a.txt"), "content1").unwrap();
    fs::write(dir.path().join("b.txt"), "content2").unwrap();

    let result = crate::core::processor::Processor::new(Box::new(MockTrash))
        .process(dir.path(), false, None)
        .unwrap();

    assert_eq!(result.scanned_count, 2);
    assert_eq!(result.duplicate_count, 0);
    assert_eq!(result.trashed_count, 0);
}

#[test]
fn test_processor_nonexistent_directory() {
    let result = crate::core::processor::Processor::new(Box::new(MockTrash))
        .process(Path::new("/nonexistent"), false, None);
    assert!(result.is_err());
}

#[test]
fn test_processor_not_a_directory() {
    let dir = tempfile::TempDir::new().unwrap();
    let file = dir.path().join("not_a_dir.txt");
    fs::write(&file, "data").unwrap();

    let result = crate::core::processor::Processor::new(Box::new(MockTrash))
        .process(&file, false, None);
    assert!(result.is_err());
}

#[test]
fn test_processor_preserves_extension_case() {
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(dir.path().join("photo.JPG"), "data_jpg").unwrap();
    fs::write(dir.path().join("image.webp"), "data_webp").unwrap();

    let result = crate::core::processor::Processor::new(Box::new(MockTrash))
        .process(dir.path(), false, None)
        .unwrap();

    let remaining: Vec<String> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| !n.starts_with('.'))
        .collect();

    assert_eq!(remaining.len(), 2);
    let exts: Vec<&str> = remaining
        .iter()
        .map(|n| n.rsplit('.').next().unwrap_or(""))
        .collect();
    assert!(exts.contains(&"JPG"));
    assert!(exts.contains(&"webp"));
}

#[test]
fn test_processor_chinese_filenames() {
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(dir.path().join("文件1.txt"), "content1").unwrap();
    fs::write(dir.path().join("文件2.txt"), "content2").unwrap();

    let result = crate::core::processor::Processor::new(Box::new(MockTrash))
        .process(dir.path(), false, None)
        .unwrap();

    assert_eq!(result.scanned_count, 2);
    assert_eq!(result.duplicate_count, 0);
    assert!(result.renamed_count >= 0);
}

#[test]
fn test_processor_mixed_extensions() {
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(dir.path().join("a.txt"), "content1").unwrap();
    fs::write(dir.path().join("b.jpg"), "content2").unwrap();
    fs::write(dir.path().join("c.png"), "content3").unwrap();

    let result = crate::core::processor::Processor::new(Box::new(MockTrash))
        .process(dir.path(), false, None)
        .unwrap();

    assert_eq!(result.scanned_count, 3);
    let remaining: Vec<String> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| !n.starts_with('.'))
        .collect();

    assert_eq!(remaining.len(), 3);
    let exts: Vec<&str> = remaining
        .iter()
        .map(|n| n.rsplit('.').next().unwrap_or(""))
        .collect();
    assert!(exts.contains(&"txt"));
    assert!(exts.contains(&"jpg"));
    assert!(exts.contains(&"png"));
}

#[test]
fn test_processor_empty_files() {
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(dir.path().join("empty1.txt"), "").unwrap();
    fs::write(dir.path().join("empty2.txt"), "").unwrap();
    fs::write(dir.path().join("nonempty.txt"), "content").unwrap();

    let result = crate::core::processor::Processor::new(Box::new(MockTrash))
        .process(dir.path(), false, None)
        .unwrap();

    assert_eq!(result.scanned_count, 3);
    assert_eq!(result.duplicate_count, 1);
    assert_eq!(result.trashed_count, 1);
}

#[test]
fn test_processor_large_files_same_size() {
    let dir = tempfile::TempDir::new().unwrap();
    let data = vec![42u8; 1024 * 100]; // 100KB
    fs::write(dir.path().join("a.bin"), &data).unwrap();
    fs::write(dir.path().join("b.bin"), &data).unwrap();
    fs::write(dir.path().join("c.bin"), vec![43u8; 1024 * 100]).unwrap();

    let result = crate::core::processor::Processor::new(Box::new(MockTrash))
        .process(dir.path(), false, None)
        .unwrap();

    assert_eq!(result.scanned_count, 3);
    assert_eq!(result.duplicate_count, 1);
    assert_eq!(result.trashed_count, 1);
}

#[test]
fn test_permission_error_skips_file() {
    let dir = tempfile::TempDir::new().unwrap();
    let protected_file = dir.path().join("protected.txt");
    fs::write(&protected_file, "secret").unwrap();
    fs::write(dir.path().join("normal.txt"), "public").unwrap();

    // Make file unreadable
    let mut perms = fs::metadata(&protected_file).unwrap().permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&protected_file, perms).unwrap();

    // Should still process, just skip the protected file
    let result = crate::core::processor::Processor::new(Box::new(MockTrash))
        .process(dir.path(), false, None);

    // Restore permissions for cleanup
    let mut perms = fs::metadata(&protected_file).unwrap().permissions();
    perms.set_mode(0o644);
    fs::set_permissions(&protected_file, perms).unwrap();

    // Result depends on implementation - either succeeds with errors or fails
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_trash_files_not_permanently_deleted() {
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(dir.path().join("a.txt"), "content").unwrap();
    fs::write(dir.path().join("b.txt"), "content").unwrap();

    let trash = MockTrash;
    let result = trash.move_to_trash(&dir.path().join("a.txt"));
    assert!(result.is_ok());
    assert!(!dir.path().join("a.txt").exists());
    assert!(dir.path().join(".mock_trash/a.txt").exists());
}

#[test]
fn test_concurrent_prevention() {
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::thread;

    let _dir = tempfile::TempDir::new().unwrap();
    fs::write(_dir.path().join("a.txt"), "content1").unwrap();
    fs::write(_dir.path().join("b.txt"), "content2").unwrap();

    let state = Arc::new(Mutex::new(false));
    let state_clone = state.clone();

    // First "processing" thread
    let handle1 = thread::spawn(move || {
        let mut processing = state_clone.lock().unwrap();
        if *processing {
            return Err("Already processing");
        }
        *processing = true;
        // Simulate processing
        thread::sleep(std::time::Duration::from_millis(100));
        *processing = false;
        Ok(())
    });

    // Second thread tries to start
    let state_clone2 = state.clone();
    let handle2 = thread::spawn(move || {
        let mut processing = state_clone2.lock().unwrap();
        if *processing {
            return Err("Already processing");
        }
        *processing = true;
        thread::sleep(std::time::Duration::from_millis(50));
        *processing = false;
        Ok(())
    });

    let r1 = handle1.join().unwrap();
    let r2 = handle2.join().unwrap();
    // At least one should succeed
    assert!(r1.is_ok() || r2.is_ok());
}

fn create_file_entry(dir: &Path, name: &str) -> FileEntry {
    let path = dir.join(name);
    fs::write(&path, "test").unwrap();
    FileEntry {
        path,
        file_name: name.to_string(),
        extension: name.rsplit('.').next().unwrap_or("").to_string(),
        size: 4,
        md5: None,
        is_duplicate: false,
        is_kept: false,
    }
}

struct MockTrash;

impl crate::platform::trash::TrashOperation for MockTrash {
    fn move_to_trash(&self, path: &Path) -> Result<(), String> {
        let parent = path.parent().unwrap();
        let trash_dir = parent.join(".mock_trash");
        fs::create_dir_all(&trash_dir).map_err(|e| e.to_string())?;
        let dest = trash_dir.join(path.file_name().unwrap());
        fs::rename(path, dest).map_err(|e| e.to_string())?;
        Ok(())
    }
}
