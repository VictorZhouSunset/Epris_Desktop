use chrono::{SecondsFormat, Utc};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::Emitter;

fn timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn env_install_log_path(base_dir: &Path) -> PathBuf {
    base_dir.join("logs").join("env-install.log")
}

fn env_check_log_path(base_dir: &Path) -> PathBuf {
    base_dir.join("logs").join("env-check.log")
}

fn append_line(path: &Path, line: &str) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut f = match OpenOptions::new().create(true).append(true).open(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "[Epris] Failed to open env install log {}: {}",
                path.to_string_lossy(),
                e
            );
            return;
        }
    };
    let _ = writeln!(f, "[{}] {}", timestamp(), line);
}

pub fn toolchain_dir_to_app_data_dir(toolchain_dir: &Path) -> Option<PathBuf> {
    toolchain_dir.parent().map(|p| p.to_path_buf())
}

pub fn log_env_install(
    window: Option<&tauri::Window>,
    app_data_dir: Option<&Path>,
    workspace_path: Option<&Path>,
    line: &str,
) {
    if let Some(w) = window {
        let _ = w.emit("env_install_log", line.to_string());
    }

    if let Some(app_dir) = app_data_dir {
        append_line(&env_install_log_path(app_dir), line);
    }
    if let Some(ws) = workspace_path {
        append_line(&env_install_log_path(ws), line);
    }
}

pub fn log_env_check(app_data_dir: Option<&Path>, workspace_path: Option<&Path>, line: &str) {
    if let Some(app_dir) = app_data_dir {
        append_line(&env_check_log_path(app_dir), line);
    }
    if let Some(ws) = workspace_path {
        append_line(&env_check_log_path(ws), line);
    }
}

pub fn begin_env_install(
    window: Option<&tauri::Window>,
    app_data_dir: Option<&Path>,
    workspace_path: Option<&Path>,
    provider: &str,
    workspace: &str,
) {
    log_env_install(
        window,
        app_data_dir,
        workspace_path,
        &format!("=== Env install started (provider: {}, workspace: {}) ===", provider, workspace),
    );
}
