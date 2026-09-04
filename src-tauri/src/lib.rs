pub mod commands;
pub mod core;
pub mod platform;

#[cfg(test)]
mod integration_tests;

use platform::trash::TrashOperation;

#[cfg(target_os = "linux")]
use platform::linux_impl::LinuxTrash;

#[cfg(target_os = "macos")]
use platform::macos_impl::MacosTrash;

#[cfg(target_os = "windows")]
use platform::windows_impl::WindowsTrash;

/// Get the platform-specific trash implementation.
pub fn get_trash_impl() -> Box<dyn TrashOperation> {
    #[cfg(target_os = "linux")]
    {
        Box::new(LinuxTrash::new())
    }

    #[cfg(target_os = "macos")]
    {
        Box::new(MacosTrash::new())
    }

    #[cfg(target_os = "windows")]
    {
        Box::new(WindowsTrash::new())
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        compile_error!("Unsupported platform")
    }
}

/// Run the Tauri application.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(commands::process::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::process::process_directory,
            commands::process::install_context_menu,
            commands::process::uninstall_context_menu
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
