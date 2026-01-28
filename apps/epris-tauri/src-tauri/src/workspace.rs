use serde::{Serialize, Deserialize};
use std::path::Path;
use std::io::Write;
use crate::utils;
use crate::state_manager::StateManager;

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub path: String,
    pub exists: bool,
}

pub fn mirror_workspace(template_path: &Path, target_path: &Path) -> Result<(), String> {
    println!("[Epris] Template Path: {:?} (exists: {}, is_dir: {})", 
        template_path, template_path.exists(), template_path.is_dir());
    
    if !template_path.exists() {
        return Err(format!("[Epris] Template directory not found at: {:?}", template_path));
    }
    if !template_path.is_dir() {
        return Err(format!("[Epris] Template path is not a directory: {:?}", template_path));
    }

    // POLICY: Initialize workspace if package.json is missing (more robust than checking directory)
    let target_pkg = target_path.join("package.json");
    if !target_pkg.exists() {
        println!("[Epris] Initializing new workspace at: {:?}", target_path);
        // Remove any partial/corrupt workspace first
        if target_path.exists() {
            std::fs::remove_dir_all(target_path)
                .map_err(|e| format!("[Epris] Failed to clean corrupt workspace: {}", e))?;
        }
        let mut options = fs_extra::dir::CopyOptions::new();
        options.copy_inside = true;
        fs_extra::dir::copy(template_path, target_path, &options)
            .map_err(|e| format!("[Epris] Failed to copy workspace template: {}", e))?;
        
        // Delete copied node_modules (contains broken symlinks to template's .pnpm store)
        let copied_node_modules = target_path.join("node_modules");
        if copied_node_modules.exists() {
            println!("[Epris] Removing copied node_modules (will reinstall fresh)");
            let _ = std::fs::remove_dir_all(&copied_node_modules);
        }
    } else {
        println!("[Epris] Workspace already exists at: {:?}", target_path);
        // Always sync package.json from template (for dev updates)
        let template_pkg = template_path.join("package.json");
        if template_pkg.exists() {
            std::fs::copy(&template_pkg, &target_pkg)
                .map_err(|e| format!("[Epris] Failed to sync package.json: {}", e))?;
            println!("[Epris] Synced package.json from template");
        }
    }

    // Sync pnpm-lock.yaml and detect changes
    let template_lock = template_path.join("pnpm-lock.yaml");
    let target_lock = target_path.join("pnpm-lock.yaml");
    let mut lock_changed = false;

    if template_lock.exists() {
        if target_lock.exists() {
            let template_content = std::fs::read(&template_lock).unwrap_or_default();
            let target_content = std::fs::read(&target_lock).unwrap_or_default();
            if template_content != target_content {
                println!("[Epris] pnpm-lock.yaml changed, triggering reinstall");
                lock_changed = true;
            }
        } else {
            lock_changed = true;
        }
        std::fs::copy(&template_lock, &target_lock)
            .map_err(|e| format!("[Epris] Failed to sync pnpm-lock.yaml: {}", e))?;
    }

    let logs_path = target_path.join("logs");
    if !logs_path.exists() {
        std::fs::create_dir_all(&logs_path).map_err(|e| format!("[Epris] Failed to create logs dir: {}", e))?;
    }

    let node_modules_path = target_path.join("node_modules");
    if !node_modules_path.exists() || lock_changed {
        println!("[Epris] Installing/Updating dependencies in workspace (lock_changed: {})...", lock_changed);
        
        let mut log_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(logs_path.join("bootstrap.log"))
            .map_err(|e| format!("[Epris] Failed to open bootstrap.log: {}", e))?;
        
        // Write timestamp entry
        let _ = writeln!(log_file, "\n--- Bootstrap Log [{:?}] ---", std::time::SystemTime::now());
        
        // Run pnpm install and captured stdout/stderr
        let status = utils::create_shell_command("pnpm", &["install"])
            .current_dir(target_path)
            .stdout(log_file.try_clone().map_err(|e| e.to_string())?)
            .stderr(log_file)
            .status()
            .map_err(|e| format!("[Epris] Failed to run pnpm install: {}", e))?;
            
        if !status.success() {
             return Err(format!("[Epris] pnpm install failed. Check workspace/logs/bootstrap.log for details. Exit code: {:?}", status.code()));
        }
        println!("[Epris] Dependencies installed successfully");
    }

    Ok(())
}

#[tauri::command]
pub fn get_workspace_config(_app_handle: tauri::AppHandle) -> Result<WorkspaceConfig, String> {
    let app_dir = _app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("[Epris] Failed to get app data dir: {}", e))?;

    let state_mgr = StateManager::new(app_dir);
    let state = state_mgr.read();
    let Some(active_id) = state.active_project_id else {
        return Ok(WorkspaceConfig {
            path: String::new(),
            exists: false,
        });
    };
    let Some(p) = state.projects.get(&active_id) else {
        return Ok(WorkspaceConfig {
            path: String::new(),
            exists: false,
        });
    };

    let p_path = std::path::Path::new(&p.path);
    Ok(WorkspaceConfig {
        path: p.path.clone(),
        exists: p_path.exists(),
    })
}
