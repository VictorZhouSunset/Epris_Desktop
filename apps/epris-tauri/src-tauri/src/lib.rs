mod workspace;
mod projects;
mod preview;
mod opencode;
mod gate;
mod snapshot;
mod export;
mod video_config;
mod utils;

use std::sync::Mutex;
use tauri::Manager;

pub use gate::GateResult;

// Snapshot system and loggers moved to snapshot.rs and opencode.rs

pub mod state_manager;
pub mod environment;
pub mod skills;
pub mod provider;

use crate::state_manager::{StateManager, AppStateStore};
use crate::environment::{EnvironmentManager, EnvironmentStatus};
// Duplicate Manager import removed


// ============================================================================
// Snapshot System - Data Structures
// ============================================================================

// DAG management moved to snapshot.rs

// Workspace management logic moved to workspace.rs

// Workspace helper and config moved to workspace.rs

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// Preview server logic moved to preview.rs

// ============================================================================
// OpenCode Integration
// ============================================================================

// OpenCode integration and logic moved to opencode.rs

// ============================================================================
// Gate Validation System
// ============================================================================

// Gate validation logic moved to gate.rs
// GateResult struct moved to gate.rs (re-exported at crate root)


// ============================================================================
// Snapshot commands moved to snapshot.rs

// Snapshot and Gate history logic moved to snapshot.rs and gate.rs

// Video export logic moved to export.rs

pub fn trigger_cleanup(app_handle: &tauri::AppHandle) {
    println!("[Epris] Initializing process cleanup...");
    
    // Clean up Preview Server
    if let Ok(mut server) = app_handle.state::<Mutex<preview::PreviewServerState>>().lock() {
        if let Some(mut child) = server.process.take() {
            utils::kill_process_tree(&mut child);
        }
    }

    // Clean up OpenCode
    if let Ok(mut oc) = app_handle.state::<Mutex<opencode::OpenCodeState>>().lock() {
        if let Some(mut child) = oc.process.take() {
            utils::kill_process_tree(&mut child);
        }
    }
    
    // Specialized port based cleanup
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let script = "
            $ports = @(3030, 4096);
            foreach ($port in $ports) {
                try {
                    $proc = Get-NetTCPConnection -LocalPort $port -ErrorAction Stop;
                    Stop-Process -Id $proc.OwningProcess -Force -ErrorAction SilentlyContinue;
                } catch {}
            }
        ";
        let _ = std::process::Command::new("powershell")
            .arg("-NoProfile")
            .arg("-Command")
            .arg(script)
            .creation_flags(0x08000000)
            .status();
    }

    println!("[Epris] Cleanup complete.");
}

#[tauri::command]
async fn get_environment_status(app_handle: tauri::AppHandle, provider: String, workspace_path: Option<String>) -> Result<EnvironmentStatus, String> {
    let app_dir = app_handle.path().app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    tauri::async_runtime::spawn_blocking(move || {
        let env_manager = EnvironmentManager::new(app_dir);
        Ok(env_manager.check_environment(&provider, workspace_path.as_deref()))
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
fn get_app_state(app_handle: tauri::AppHandle) -> Result<AppStateStore, String> {
    let app_dir = app_handle.path().app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let state_manager = StateManager::new(app_dir);
    Ok(state_manager.read())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(preview::PreviewServerState::default()))
        .manage(Mutex::new(opencode::OpenCodeState::default()))
        .manage(Mutex::new(export::ExportState::default()))
        .setup(|app| {
            // Signal Handler for Ctrl+C (Terminal Exit)
            let handle = app.handle().clone();
            ctrlc::set_handler(move || {
                println!("\n[Epris] Keyboard interrupt detected (Ctrl-C).");
                trigger_cleanup(&handle);
                std::process::exit(0);
            }).expect("Error setting Ctrl-C handler");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet, 
            workspace::get_workspace_config,
            projects::get_projects_overview,
            projects::get_active_project_config,
            projects::get_default_projects_root,
            projects::pick_projects_root,
            projects::set_projects_root,
            projects::create_project,
            projects::set_active_project,
            projects::delete_project,
            video_config::get_video_config,
            video_config::set_video_config,
            preview::start_preview_server,
            preview::stop_preview_server,
            opencode::start_opencode,
            // opencode::stop_opencode, // Use provider::stop_opencode logic or similar? provider.rs has duplicates. 
            // Stick to opencode::stop_opencode for now as duplicates were removed from provider.rs
            opencode::stop_opencode, 
            opencode::send_prompt,
            opencode::clear_session,
            gate::run_gate,
            export::export_video,
            snapshot::delete_snapshot_tree,
            snapshot::get_dag_head,
            snapshot::save_dag_head,
            snapshot::auto_save_snapshot,
            snapshot::manual_save_snapshot,
            snapshot::checkout_snapshot,
            snapshot::get_snapshot_dag,
            snapshot::list_snapshots,
            snapshot::update_snapshot_metadata,
            snapshot::get_gate_history,
            snapshot::check_unsaved_changes,
            snapshot::clear_snapshot_history,
            snapshot::save_snapshot_layout,
            // Environment & Gemini
            get_environment_status,
            get_app_state,
            environment::install_missing_dependencies,
            environment::get_gemini_auth_status,
            environment::open_gemini_login,
            environment::set_gemini_api_key,
            environment::open_gemini_auth_terminal,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::ExitRequested { .. } = event {
            trigger_cleanup(app_handle);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_workspace_initialization() {
        let tmp = tempdir().expect("Failed to create temp dir");
        let template_path = tmp.path().join("template");
        let target_path = tmp.path().join("workspace");
        
        std::fs::create_dir(&template_path).unwrap();
        std::fs::write(template_path.join("dummy.txt"), "hello").unwrap();

        workspace::mirror_workspace(&template_path, &target_path).expect("Should succeed");
        
        assert!(target_path.exists());
        assert!(target_path.join("dummy.txt").exists());
        assert!(target_path.join("logs").exists());
    }

    #[test]
    fn test_gate_result_has_durations() {
        let result = GateResult {
            success: true,
            passed: true,
            typecheck_passed: true,
            typecheck: true,
            smoke_0_passed: true,
            smoke_0: true,
            smoke_mid_passed: true,
            smoke_mid: true,
            error_output: String::new(),
            error: None,
            attempts: 1,
            // New duration fields
            duration_typecheck_sec: 1.5,
            duration_smoke_0_sec: 2.0,
            duration_smoke_mid_sec: 3.5,
            total_duration_sec: 7.0,
        };
        assert_eq!(result.total_duration_sec, 7.0);
    }
}
