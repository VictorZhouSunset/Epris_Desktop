mod workspace;
mod projects;
mod preview;
mod opencode;
mod gate;
mod snapshot;
mod export;
mod video_config;
mod utils;
mod assets;
mod stt;
mod sandbox;
mod toolchain;
mod maintenance;
mod install_log;
mod agent_plan;
mod ui_props;
mod ui_objects;

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::Manager;
use tokio::time::sleep;

pub use gate::GateResult;

// Snapshot system and loggers moved to snapshot.rs and opencode.rs

pub mod state_manager;
pub mod environment;
pub mod skills;
pub mod provider;

use crate::state_manager::{StateManager, AppStateStore};
use crate::environment::{EnvironmentManager, EnvironmentStatus};
// Duplicate Manager import removed

const ENV_STATUS_CACHE_TTL: Duration = Duration::from_millis(1500);
const ENV_STATUS_LOCK_POLL_MS: u64 = 25;

#[derive(Clone)]
struct EnvStatusCacheEntry {
    checked_at: Instant,
    status: EnvironmentStatus,
}

#[derive(Default)]
struct EnvStatusDedupState {
    cache: HashMap<String, EnvStatusCacheEntry>,
    in_flight: HashSet<String>,
}

static ENV_STATUS_DEDUP_STATE: OnceLock<Mutex<EnvStatusDedupState>> = OnceLock::new();

fn env_status_dedup_state() -> &'static Mutex<EnvStatusDedupState> {
    ENV_STATUS_DEDUP_STATE.get_or_init(|| Mutex::new(EnvStatusDedupState::default()))
}

fn env_status_key(provider: &str, workspace_path: Option<&str>) -> String {
    format!("{}::{}", provider, workspace_path.unwrap_or("<none>"))
}

fn maybe_migrate_legacy_app_data_dir(app_handle: &tauri::AppHandle) {
    let Ok(current_dir) = app_handle.path().app_local_data_dir() else {
        return;
    };
    let current_state = current_dir.join("state.json");
    if current_state.exists() {
        return;
    }

    let Some(parent) = current_dir.parent() else {
        return;
    };

    // Best-effort migration for app renames (e.g. productName changes).
    // If the new app data dir is empty/missing but an old one exists, copy/move it forward.
    let legacy_candidates = ["epris-tauri", "Epris", "epris"];
    for legacy_name in legacy_candidates {
        if let Some(n) = current_dir.file_name() {
            if n == legacy_name {
                continue;
            }
        }

        let legacy_dir = parent.join(legacy_name);
        let legacy_state = legacy_dir.join("state.json");
        if !legacy_state.exists() {
            continue;
        }

        if !current_dir.exists() {
            if std::fs::rename(&legacy_dir, &current_dir).is_ok() {
                println!(
                    "[Epris] Migrated app data dir (rename): {} -> {}",
                    legacy_dir.to_string_lossy(),
                    current_dir.to_string_lossy()
                );
                return;
            }
        }

        // Fallback: copy inside, keep legacy dir in place (safer).
        let _ = std::fs::create_dir_all(&current_dir);
        let mut options = fs_extra::dir::CopyOptions::new();
        options.copy_inside = true;
        options.overwrite = false;
        let _ = fs_extra::dir::copy(&legacy_dir, &current_dir, &options);

        if current_dir.join("state.json").exists() {
            println!(
                "[Epris] Migrated app data dir (copy): {} -> {}",
                legacy_dir.to_string_lossy(),
                current_dir.to_string_lossy()
            );
            return;
        }
    }
}

fn auto_save_active_project_checkpoint(app_handle: &tauri::AppHandle, reason: &str) {
    let app_dir = match app_handle.path().app_local_data_dir() {
        Ok(p) => p,
        Err(_) => return,
    };
    let state_mgr = StateManager::new(app_dir);
    let state = state_mgr.read();
    let Some(active_id) = state.active_project_id else {
        return;
    };
    let Some(p) = state.projects.get(&active_id) else {
        return;
    };
    snapshot::auto_save_checkpoint_best_effort(&p.path, reason);
}


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

#[tauri::command]
fn append_updater_log(app_handle: tauri::AppHandle, line: String) -> Result<(), String> {
    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let logs_dir = app_dir.join("logs");
    std::fs::create_dir_all(&logs_dir).map_err(|e| e.to_string())?;

    let path = logs_dir.join("updater.log");
    let now = chrono::Utc::now().to_rfc3339();
    let entry = format!("[{}] {}\n", now, line.trim_end());

    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    f.write_all(entry.as_bytes()).map_err(|e| e.to_string())?;
    Ok(())
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

    // Save a best-effort checkpoint so users can recover from partial runs.
    auto_save_active_project_checkpoint(app_handle, "App exit");
    
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

    // Clean up Gate validation (pnpm processes)
    let _ = gate::stop_gate_validation();
    
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
    let total_started = Instant::now();
    let key = env_status_key(&provider, workspace_path.as_deref());
    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let app_dir_for_log = app_handle.path().app_local_data_dir().ok();
    let ws_for_log = workspace_path.as_deref().map(std::path::Path::new);

    let log_line = |line: &str| {
        crate::install_log::log_env_check(app_dir_for_log.as_deref(), ws_for_log, line);
        println!("[Epris][EnvCheck] {}", line);
    };

    // Fast return from short-lived cache.
    {
        let guard = match env_status_dedup_state().lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        if let Some(entry) = guard.cache.get(&key) {
            if entry.checked_at.elapsed() <= ENV_STATUS_CACHE_TTL {
                log_line(&format!(
                    "get_environment_status provider={} cache=hit total={} ms",
                    provider,
                    total_started.elapsed().as_millis()
                ));
                return Ok(entry.status.clone());
            }
        }
    }

    // In-flight dedup: if another identical check is running, wait for it and then reuse cache.
    loop {
        let should_wait = {
            let mut guard = match env_status_dedup_state().lock() {
                Ok(g) => g,
                Err(e) => e.into_inner(),
            };

            if let Some(entry) = guard.cache.get(&key) {
                if entry.checked_at.elapsed() <= ENV_STATUS_CACHE_TTL {
                    log_line(&format!(
                        "get_environment_status provider={} cache=hit_after_wait total={} ms",
                        provider,
                        total_started.elapsed().as_millis()
                    ));
                    return Ok(entry.status.clone());
                }
            }

            if !guard.in_flight.contains(&key) {
                guard.in_flight.insert(key.clone());
                false
            } else {
                true
            }
        };

        if !should_wait {
            break;
        }
        sleep(Duration::from_millis(ENV_STATUS_LOCK_POLL_MS)).await;
    }

    let provider_for_compute = provider.clone();
    let workspace_for_compute = workspace_path.clone();

    let compute_result: Result<EnvironmentStatus, String> = match tauri::async_runtime::spawn_blocking(move || {
        let env_manager = EnvironmentManager::new(app_dir);
        Ok(env_manager.check_environment(&provider_for_compute, workspace_for_compute.as_deref()))
    })
    .await
    {
        Ok(v) => v,
        Err(e) => Err(e.to_string()),
    };

    {
        let mut guard = match env_status_dedup_state().lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        guard.in_flight.remove(&key);
        if let Ok(status) = &compute_result {
            guard.cache.insert(
                key.clone(),
                EnvStatusCacheEntry {
                    checked_at: Instant::now(),
                    status: status.clone(),
                },
            );
        }
        // Keep cache bounded and fresh.
        guard
            .cache
            .retain(|_, entry| entry.checked_at.elapsed() <= Duration::from_secs(30));
    }

    log_line(&format!(
        "get_environment_status provider={} cache=miss total={} ms",
        provider,
        total_started.elapsed().as_millis()
    ));
    compute_result
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
    let mut builder = tauri::Builder::default().plugin(tauri_plugin_opener::init());

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    }

    let app = builder
        .manage(Mutex::new(preview::PreviewServerState::default()))
        .manage(Mutex::new(opencode::OpenCodeState::default()))
        .manage(Mutex::new(export::ExportState::default()))
        .setup(|app| {
            maybe_migrate_legacy_app_data_dir(&app.handle());
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
            projects::rename_project,
            video_config::get_video_config,
            video_config::set_video_config,
            preview::start_preview_server,
            preview::stop_preview_server,
            opencode::start_opencode,
            // opencode::stop_opencode, // Use provider::stop_opencode logic or similar? provider.rs has duplicates. 
            // Stick to opencode::stop_opencode for now as duplicates were removed from provider.rs
            opencode::stop_opencode, 
            opencode::cancel_current_run,
            opencode::send_prompt,
            opencode::send_ui_prompt,
            opencode::clear_session,
            gate::run_gate,
            gate::stop_gate_validation,
            export::export_video,
            snapshot::delete_snapshot_tree,
            snapshot::get_dag_head,
            snapshot::save_dag_head,
            snapshot::auto_save_snapshot,
            snapshot::auto_save_checkpoint,
            snapshot::manual_save_snapshot,
            snapshot::checkout_snapshot,
            snapshot::get_snapshot_dag,
            snapshot::list_snapshots,
            snapshot::update_snapshot_metadata,
            snapshot::get_gate_history,
            snapshot::check_unsaved_changes,
            snapshot::clear_snapshot_history,
            snapshot::save_snapshot_layout,
            snapshot::restore_pre_flight_backup,
            // Environment & Gemini
            get_environment_status,
            get_app_state,
            append_updater_log,
            environment::install_missing_dependencies,
            environment::install_js_packages,
            environment::link_workspace_dependencies,
            environment::get_baseline_packages_info,
            environment::install_baseline_packages,
            environment::cancel_env_install,
            environment::get_gemini_auth_status,
            environment::open_gemini_login,
            environment::set_gemini_api_key,
            environment::open_gemini_auth_terminal,
            provider::stop_provider_cli,
            maintenance::reset_user_data,
            maintenance::set_debug_force_dependency_request,
            maintenance::set_baseline_packages_ack,
            agent_plan::get_agent_plan_state,
            assets::list_assets,
            assets::upload_asset,
            assets::delete_asset,
            assets::rename_asset,
            ui_props::get_epris_controls,
            ui_props::get_epris_props,
            ui_props::set_epris_props,
            ui_objects::scan_epris_objects,
            stt::get_stt_status,
            stt::install_whispercpp,
            stt::transcribe_whispercpp,
            stt::set_stt_model,
            stt::set_stt_task,
            stt::cancel_stt,
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
