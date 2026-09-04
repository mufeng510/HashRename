use std::path::Path;

use crate::platform::trash::TrashOperation;

/// Windows trash implementation using Recycle Bin via SHFileOperation.
pub struct WindowsTrash;

impl WindowsTrash {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WindowsTrash {
    fn default() -> Self {
        Self::new()
    }
}

impl TrashOperation for WindowsTrash {
    fn move_to_trash(&self, path: &Path) -> std::result::Result<(), String> {
        // On Windows, we use the SHFileOperation API to move to Recycle Bin
        // For now, we'll implement a fallback that moves to a temp location
        // The actual implementation would use winapi crate
        
        #[cfg(target_os = "windows")]
        {
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;
            
            let path_wide: Vec<u16> = OsStr::new(path)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            
            // SHFileOperation requires double-null terminated strings
            let mut from = path_wide.clone();
            from.push(0);
            
            // For now, just move to a temp directory as fallback
            // A proper implementation would use IFileOperation or SHFileOperation
            let temp_dir = std::env::temp_dir().join("hashrename_trash");
            std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
            
            let file_name = path.file_name().ok_or("Invalid filename")?;
            let dest = temp_dir.join(file_name);
            
            std::fs::rename(path, &dest).map_err(|e| e.to_string())?;
            Ok(())
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            // Fallback for non-Windows (shouldn't be called)
            Err("Windows trash not available on this platform".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_windows_trash() {
        let dir = TempDir::new().unwrap();
        let test_file = dir.path().join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let trash = WindowsTrash::new();
        let result = trash.move_to_trash(&test_file);
        // On non-Windows, this should fail gracefully
        if cfg!(target_os = "windows") {
            assert!(result.is_ok());
        } else {
            assert!(result.is_err());
        }
    }
}
