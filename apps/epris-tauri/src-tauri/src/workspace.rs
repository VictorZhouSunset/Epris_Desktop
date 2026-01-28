use serde::{Serialize, Deserialize};
use std::path::Path;
use crate::state_manager::StateManager;
use tauri::Manager;
use walkdir::WalkDir;

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
        std::fs::create_dir_all(target_path)
            .map_err(|e| format!("[Epris] Failed to create workspace dir: {}", e))?;

        // Copy everything from template except node_modules (often huge + contains broken links when copied).
        for entry in WalkDir::new(template_path).follow_links(false) {
            let entry = entry.map_err(|e| format!("[Epris] Failed to walk template: {}", e))?;
            let src = entry.path();
            let rel = src
                .strip_prefix(template_path)
                .map_err(|e| format!("[Epris] Failed to relativize template path: {}", e))?;

            if rel.as_os_str().is_empty() {
                continue;
            }

            if let Some(first) = rel.components().next() {
                if first.as_os_str() == "node_modules" {
                    continue;
                }
            }

            let dst = target_path.join(rel);
            if entry.file_type().is_dir() {
                std::fs::create_dir_all(&dst)
                    .map_err(|e| format!("[Epris] Failed to create dir {:?}: {}", dst, e))?;
            } else if entry.file_type().is_file() {
                if let Some(parent) = dst.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("[Epris] Failed to create parent dir {:?}: {}", parent, e))?;
                }
                std::fs::copy(src, &dst)
                    .map_err(|e| format!("[Epris] Failed to copy file {:?} -> {:?}: {}", src, dst, e))?;
            }
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

    // Sync pnpm-lock.yaml (best-effort)
    let template_lock = template_path.join("pnpm-lock.yaml");
    let target_lock = target_path.join("pnpm-lock.yaml");

    if template_lock.exists() {
        std::fs::copy(&template_lock, &target_lock)
            .map_err(|e| format!("[Epris] Failed to sync pnpm-lock.yaml: {}", e))?;
    }

    let logs_path = target_path.join("logs");
    if !logs_path.exists() {
        std::fs::create_dir_all(&logs_path).map_err(|e| format!("[Epris] Failed to create logs dir: {}", e))?;
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
