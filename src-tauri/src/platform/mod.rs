pub mod trash;

#[cfg(target_os = "windows")]
pub mod windows_impl;

#[cfg(target_os = "macos")]
pub mod macos_impl;

#[cfg(target_os = "linux")]
pub mod linux_impl;

pub use trash::TrashOperation;
