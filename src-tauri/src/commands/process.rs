use std::sync::Mutex;
use tauri::State;

use crate::core::models::ProcessingResult;
use crate::core::processor::Processor;
use crate::get_trash_impl;

/// Shared app state for processing
pub struct AppState {
    pub processing: Mutex<bool>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            processing: Mutex::new(false),
        }
    }
}

#[tauri::command]
pub fn process_directory(
    directory: String,
    verbose: bool,
    state: State<'_, AppState>,
) -> Result<ProcessingResult, String> {
    // Check if already processing
    {
        let mut processing = state.processing.lock().map_err(|e| e.to_string())?;
        if *processing {
            return Err("Already processing a directory. Please wait.".to_string());
        }
        *processing = true;
    }

    let result = {
        let dir = std::path::Path::new(&directory);
        let trash = get_trash_impl();
        let processor = Processor::new(trash);
        processor.process(dir, verbose, None)
    };

    // Reset processing flag
    {
        let mut processing = state.processing.lock().map_err(|e| e.to_string())?;
        *processing = false;
    }

    result.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn install_context_menu() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
        let exe_path_str = exe_path.to_string_lossy().to_string();

        // Add to registry
        let reg_cmd = format!(
            "reg add \"HKCR\\Directory\\shell\\HashRename\" /ve /d \"Hash 去重并重命名\" /f"
        );
        Command::new("cmd")
            .args(["/C", &reg_cmd])
            .output()
            .map_err(|e| e.to_string())?;

        let command = format!("\"{}\" \"%V\"", exe_path_str);
        let reg_cmd = format!(
            "reg add \"HKCR\\Directory\\shell\\HashRename\\command\" /ve /d \"{}\" /f",
            command
        );
        Command::new("cmd")
            .args(["/C", &reg_cmd])
            .output()
            .map_err(|e| e.to_string())?;

        Ok("Context menu installed successfully. You may need to restart Explorer.".to_string())
    }

    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
        let services_dir = home.join("Library/Services");
        std::fs::create_dir_all(&services_dir).map_err(|e| e.to_string())?;

        let app_dir = std::env::current_exe()
            .map_err(|e| e.to_string())?
            .parent()
            .ok_or("Cannot determine app directory")?
            .to_path_buf();

        let script = format!(
            r#"#!/bin/bash
"{}" "$1"
"#,
            app_dir.join("hashrename").display()
        );

        let service_name = "HashRename.workflow";
        let workflow_dir = services_dir.join(service_name);
        std::fs::create_dir_all(workflow_dir.join("Contents")).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(workflow_dir.join("Contents/Workflow")).map_err(|e| e.to_string())?;

        let info_plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>NSServices</key>
    <array>
        <dict>
            <key>NSMenuItem</key>
            <dict>
                <key>default</key>
                <string>Hash 去重并重命名</string>
            </dict>
            <key>NSMessage</key>
            <string>runWorkflowAsService</string>
            <key>NSSendFileTypes</key>
            <array>
                <string>public.folder</string>
            </array>
        </dict>
    </array>
</dict>
</plist>"#;

        std::fs::write(workflow_dir.join("Contents/document.wflow"), "").map_err(|e| e.to_string())?;
        std::fs::write(workflow_dir.join("Contents/Info.plist"), info_plist).map_err(|e| e.to_string())?;

        Ok("Service installed in ~/Library/Services. You may need to log out and back in.".to_string())
    }

    #[cfg(target_os = "linux")]
    {
        let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
        let scripts_dir = home.join(".local/share/nautilus/scripts");
        std::fs::create_dir_all(&scripts_dir).map_err(|e| e.to_string())?;

        let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
        let exe_path_str = exe_path.to_string_lossy().to_string();

        let script = format!(
            r#"#!/bin/bash
"{}" "$NAUTILUS_SCRIPT_SELECTED_FILE_PATHS"
"#,
            exe_path_str
        );

        let script_path = scripts_dir.join("Hash 去重并重命名");
        std::fs::write(&script_path, script).map_err(|e| e.to_string())?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o755);
            std::fs::set_permissions(&script_path, perms).map_err(|e| e.to_string())?;
        }

        Ok("Nautilus script installed. Right-click in Files to use.".to_string())
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err("Context menu installation not supported on this platform".to_string())
    }
}

#[tauri::command]
pub fn uninstall_context_menu() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        let reg_cmd = "reg delete \"HKCR\\Directory\\shell\\HashRename\" /f";
        Command::new("cmd")
            .args(["/C", reg_cmd])
            .output()
            .map_err(|e| e.to_string())?;

        Ok("Context menu uninstalled successfully. You may need to restart Explorer.".to_string())
    }

    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
        let service_path = home.join("Library/Services/HashRename.workflow");
        if service_path.exists() {
            std::fs::remove_dir_all(&service_path).map_err(|e| e.to_string())?;
        }
        Ok("Service removed. You may need to log out and back in.".to_string())
    }

    #[cfg(target_os = "linux")]
    {
        let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
        let script_path = home.join(".local/share/nautilus/scripts/Hash 去重并重命名");
        if script_path.exists() {
            std::fs::remove_file(&script_path).map_err(|e| e.to_string())?;
        }
        Ok("Nautilus script removed.".to_string())
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err("Context menu uninstallation not supported on this platform".to_string())
    }
}
