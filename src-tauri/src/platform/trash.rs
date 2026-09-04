use std::path::Path;

/// Trait for moving files to trash/recycle bin.
pub trait TrashOperation: Send + Sync {
    fn move_to_trash(&self, path: &Path) -> std::result::Result<(), String>;
}
