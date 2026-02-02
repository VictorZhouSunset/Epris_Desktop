use crate::state_manager::StateManager;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tauri::Manager;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetUserDataPayload {
    #[serde(default)]
    pub delete_projects: bool,
}

#[derive(Debug, Serialize)]
pub struct ResetFailure {
    pub path: String,
    pub error: String,
}

#[derive(Debug, Serialize)]
pub struct ResetUserDataResult {
    pub deleted: Vec<String>,
    pub failed: Vec<ResetFailure>,
    pub projects_deleted: u32,
    pub restart_required: bool,
}

fn remove_dir_best_effort(path: &Path, deleted: &mut Vec<String>, failed: &mut Vec<ResetFailure>) {
    if !path.exists() {
        return;
    }
    match std::fs::remove_dir_all(path) {
        Ok(_) => deleted.push(path.to_string_lossy().to_string()),
        Err(e) => failed.push(ResetFailure {
            path: path.to_string_lossy().to_string(),
            error: e.to_string(),
        }),
    }
}

fn remove_file_best_effort(path: &Path, deleted: &mut Vec<String>, failed: &mut Vec<ResetFailure>) {
    if !path.exists() {
        return;
    }
    match std::fs::remove_file(path) {
        Ok(_) => deleted.push(path.to_string_lossy().to_string()),
        Err(e) => failed.push(ResetFailure {
            path: path.to_string_lossy().to_string(),
            error: e.to_string(),
        }),
    }
}

#[tauri::command]
pub fn reset_user_data(
    app_handle: tauri::AppHandle,
    payload: ResetUserDataPayload,
) -> Result<ResetUserDataResult, String> {
    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    // Read state first so we can optionally delete known project folders safely.
    let state_mgr = StateManager::new(app_dir.clone());
    let state = state_mgr.read();

    // Stop any provider CLI best-effort to reduce file locking during cleanup.
    let _ = crate::provider::stop_provider_cli();
    let _ = crate::gate::stop_gate_validation();

    let mut deleted: Vec<String> = Vec::new();
    let mut failed: Vec<ResetFailure> = Vec::new();
    let mut projects_deleted: u32 = 0;

    if payload.delete_projects {
        let mut unique: BTreeSet<PathBuf> = BTreeSet::new();
        for p in state.projects.values() {
            if p.path.trim().is_empty() {
                continue;
            }
            unique.insert(PathBuf::from(&p.path));
        }
        for path in unique {
            if path.exists() {
                match std::fs::remove_dir_all(&path) {
                    Ok(_) => {
                        projects_deleted = projects_deleted.saturating_add(1);
                        deleted.push(path.to_string_lossy().to_string());
                    }
                    Err(e) => failed.push(ResetFailure {
                        path: path.to_string_lossy().to_string(),
                        error: e.to_string(),
                    }),
                }
            }
        }
    }

    // App-local data (safe to delete; will be re-created on next launch)
    remove_dir_best_effort(&app_dir.join("toolchain"), &mut deleted, &mut failed);
    remove_dir_best_effort(&app_dir.join("logs"), &mut deleted, &mut failed);
    remove_dir_best_effort(&app_dir.join("workspace-template"), &mut deleted, &mut failed);
    remove_file_best_effort(&app_dir.join("state.json"), &mut deleted, &mut failed);

    Ok(ResetUserDataResult {
        deleted,
        failed,
        projects_deleted,
        restart_required: true,
    })
}

