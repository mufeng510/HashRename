use std::path::Path;

use crate::platform::trash::TrashOperation;

/// Linux trash implementation using XDG Base Directory Specification.
pub struct LinuxTrash;

impl LinuxTrash {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LinuxTrash {
    fn default() -> Self {
        Self::new()
    }
}

impl TrashOperation for LinuxTrash {
    fn move_to_trash(&self, path: &Path) -> std::result::Result<(), String> {
        let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
        let trash_dir = home.join(".local/share/Trash");
        let files_dir = trash_dir.join("files");
        let info_dir = trash_dir.join("info");

        std::fs::create_dir_all(&files_dir).map_err(|e| format!("Failed to create Trash dir: {}", e))?;
        std::fs::create_dir_all(&info_dir).map_err(|e| format!("Failed to create Trash info dir: {}", e))?;

        let file_name = path
            .file_name()
            .ok_or("Cannot get filename")?
            .to_string_lossy()
            .to_string();

        let dest = files_dir.join(&file_name);
        let info_path = info_dir.join(format!("{}.trashinfo", file_name));

        // Handle name collision in trash
        let final_dest = if dest.exists() {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let new_name = format!("{}_{}", file_name, timestamp);
            let new_dest = files_dir.join(&new_name);
            let new_info = info_dir.join(format!("{}.trashinfo", new_name));
            
            // Write .trashinfo first
            write_trashinfo(&new_info, path)?;
            std::fs::rename(path, &new_dest).map_err(|e| format!("Failed to move to Trash: {}", e))?;
            return Ok(());
        } else {
            dest
        };

        // Write .trashinfo file
        write_trashinfo(&info_path, path)?;

        // Move file to Trash
        std::fs::rename(path, &final_dest).map_err(|e| format!("Failed to move to Trash: {}", e))?;

        Ok(())
    }
}

fn write_trashinfo(info_path: &Path, original_path: &Path) -> std::result::Result<(), String> {
    let deletion_date = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S");
    let content = format!(
        "[Trash Info]\nPath={}\nDeletionDate={}\n",
        original_path.to_string_lossy(),
        deletion_date
    );
    std::fs::write(info_path, content).map_err(|e| format!("Failed to write trashinfo: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_linux_trash() {
        let dir = TempDir::new().unwrap();
        let test_file = dir.path().join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let trash = LinuxTrash::new();
        let result = trash.move_to_trash(&test_file);
        assert!(result.is_ok());
        assert!(!test_file.exists());
    }
}
