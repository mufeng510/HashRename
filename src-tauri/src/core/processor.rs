use std::path::Path;
use std::time::Instant;

use crate::core::errors::{HashRenameError, Result};
use crate::core::models::{ProcessingError, ProcessingResult, ProgressInfo};
use crate::core::{duplicate_detector, rename_planner, scanner};
use crate::platform::trash::TrashOperation;

/// Processor orchestrates the full dedup + rename pipeline.
pub struct Processor {
    trash: Box<dyn TrashOperation>,
}

impl Processor {
    pub fn new(trash: Box<dyn TrashOperation>) -> Self {
        Self { trash }
    }

    /// Run the full pipeline on a directory.
    pub fn process(
        &self,
        dir: &Path,
        verbose: bool,
        progress_callback: Option<&dyn Fn(ProgressInfo)>,
    ) -> Result<ProcessingResult> {
        let start = Instant::now();
        let mut errors = Vec::new();
        let mut trashed_count = 0usize;
        let mut renamed_count = 0usize;
        let mut failed_count = 0usize;

        // Stage 1: Validate
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

        // Stage 2: Scan
        let report_progress = |stage: &str, current: &str, scanned: usize, dup: usize, kept: usize, pct: f32| {
            if let Some(cb) = progress_callback {
                cb(ProgressInfo {
                    stage: stage.to_string(),
                    scanned_count: scanned,
                    duplicate_count: dup,
                    kept_count: kept,
                    current_file: current.to_string(),
                    progress_percent: pct,
                });
            }
        };

        report_progress("Scanning", "", 0, 0, 0, 0.0);
        let files = scanner::scan_directory(dir)?;
        let scanned_count = files.len();

        if scanned_count == 0 {
            return Err(HashRenameError::EmptyDirectory {
                path: dir.to_path_buf(),
            });
        }

        if verbose {
            log::info!("Scanned {} files", scanned_count);
        }

        // Stage 3: Detect duplicates
        report_progress("Hashing", "", scanned_count, 0, scanned_count, 20.0);
        let (kept_files, duplicate_groups) = duplicate_detector::detect_duplicates(files)?;
        let duplicate_count: usize = duplicate_groups.iter().map(|g| g.duplicate_files.len()).sum();
        let kept_count = kept_files.len();

        if verbose {
            log::info!(
                "Found {} duplicate groups ({} files), keeping {}",
                duplicate_groups.len(),
                duplicate_count,
                kept_count
            );
        }

        report_progress("Duplicates found", "", scanned_count, duplicate_count, kept_count, 50.0);

        // Stage 4: Move duplicates to trash
        for group in &duplicate_groups {
            for dup_path in &group.duplicate_files {
                report_progress(
                    "Moving to trash",
                    &dup_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                    scanned_count,
                    duplicate_count,
                    kept_count,
                    60.0,
                );

                match self.trash.move_to_trash(dup_path) {
                    Ok(()) => {
                        trashed_count += 1;
                        if verbose {
                            log::info!("Trashed: {}", dup_path.display());
                        }
                    }
                    Err(reason) => {
                        failed_count += 1;
                        errors.push(ProcessingError {
                            path: dup_path.clone(),
                            operation: "trash".to_string(),
                            reason,
                        });
                        log::error!("Failed to trash {}: {}", dup_path.display(), errors.last().unwrap().reason);
                    }
                }
            }
        }

        // Stage 5: Plan renames
        report_progress("Planning renames", "", scanned_count, duplicate_count, kept_count, 75.0);
        let operations = rename_planner::plan_renames(kept_files, dir)?;

        if verbose {
            log::info!("Planned {} renames", operations.len());
        }

        // Stage 6: Execute renames (temp first, then final)
        let total_ops = operations.len();
        for (i, op) in operations.iter().enumerate() {
            let pct = 75.0 + (i as f32 / total_ops as f32) * 20.0;
            report_progress(
                "Renaming",
                &op.old_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                scanned_count,
                duplicate_count,
                kept_count,
                pct,
            );

            // Phase 1: Rename to temp
            if let Err(e) = std::fs::rename(&op.old_path, &op.temporary_path) {
                failed_count += 1;
                errors.push(ProcessingError {
                    path: op.old_path.clone(),
                    operation: "temp_rename".to_string(),
                    reason: e.to_string(),
                });
                log::error!("Failed temp rename {}: {}", op.old_path.display(), e);
                continue;
            }

            // Phase 2: Rename from temp to final
            if let Err(e) = std::fs::rename(&op.temporary_path, &op.new_path) {
                // Attempt recovery: rename back to original
                let _ = std::fs::rename(&op.temporary_path, &op.old_path);
                failed_count += 1;
                errors.push(ProcessingError {
                    path: op.old_path.clone(),
                    operation: "final_rename".to_string(),
                    reason: e.to_string(),
                });
                log::error!("Failed final rename {}: {}", op.new_path.display(), e);
                continue;
            }

            renamed_count += 1;
            if verbose {
                log::info!("Renamed: {} -> {}", op.old_path.display(), op.new_path.display());
            }
        }

        // Cleanup any leftover temp files
        cleanup_temp_files(dir);

        let elapsed = start.elapsed().as_secs_f64();
        report_progress("Done", "", scanned_count, duplicate_count, kept_count, 100.0);

        Ok(ProcessingResult {
            scanned_count,
            duplicate_count,
            trashed_count,
            renamed_count,
            failed_count,
            elapsed_secs: elapsed,
            errors,
        })
    }
}

/// Remove any leftover temp files from interrupted runs.
fn cleanup_temp_files(dir: &Path) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(".hashrename_tmp_") {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    use crate::platform::trash::TrashOperation;

    struct MockTrash;

    impl TrashOperation for MockTrash {
        fn move_to_trash(&self, path: &Path) -> std::result::Result<(), String> {
            // Move to a "trash" directory inside the temp dir
            let parent = path.parent().unwrap();
            let trash_dir = parent.join(".mock_trash");
            fs::create_dir_all(&trash_dir).map_err(|e| e.to_string())?;
            let dest = trash_dir.join(path.file_name().unwrap());
            fs::rename(path, dest).map_err(|e| e.to_string())?;
            Ok(())
        }
    }

    #[test]
    fn test_process_empty_directory() {
        let dir = TempDir::new().unwrap();
        let processor = Processor::new(Box::new(MockTrash));
        let result = processor.process(dir.path(), false, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_process_with_duplicates() {
        let dir = TempDir::new().unwrap();
        // Create duplicate files
        fs::write(dir.path().join("a.txt"), "identical").unwrap();
        fs::write(dir.path().join("b.txt"), "identical").unwrap();
        fs::write(dir.path().join("c.txt"), "different").unwrap();

        let processor = Processor::new(Box::new(MockTrash));
        let result = processor.process(dir.path(), false, None).unwrap();

        assert_eq!(result.scanned_count, 3);
        assert_eq!(result.duplicate_count, 1);
        assert_eq!(result.trashed_count, 1);
        assert!(result.renamed_count > 0);
        assert_eq!(result.failed_count, 0);

        // Verify a.txt or b.txt was trashed
        let remaining: Vec<String> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| !n.starts_with('.'))
            .collect();

        assert_eq!(remaining.len(), 2);
    }

    #[test]
    fn test_process_no_duplicates() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.txt"), "content1").unwrap();
        fs::write(dir.path().join("b.txt"), "content2").unwrap();

        let processor = Processor::new(Box::new(MockTrash));
        let result = processor.process(dir.path(), false, None).unwrap();

        assert_eq!(result.scanned_count, 2);
        assert_eq!(result.duplicate_count, 0);
        assert_eq!(result.trashed_count, 0);
        assert!(result.renamed_count >= 0);
    }

    #[test]
    fn test_process_nonexistent_directory() {
        let processor = Processor::new(Box::new(MockTrash));
        let result = processor.process(Path::new("/nonexistent"), false, None);
        assert!(result.is_err());
    }
}
