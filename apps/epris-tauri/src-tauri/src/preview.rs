use std::process::Child;
use std::sync::Mutex;
use crate::utils;
use std::io::Write;
use std::process::Stdio;
use tauri::Manager;

fn ensure_preview_scaffold(workspace_path: &std::path::Path) -> Result<(), String> {
    let src_dir = workspace_path.join("src");
    if !src_dir.exists() {
        std::fs::create_dir_all(&src_dir).map_err(|e| format!("Failed to create src dir: {}", e))?;
    }

    // If these files are missing, Vite preview will fail even if node_modules exists.
    // Only write missing files; do not overwrite user edits.
    const INDEX_TSX: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../workspace-template/src/index.tsx"
    ));
    const PREVIEW_TSX: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../workspace-template/src/Preview.tsx"
    ));
    const COMPOSITION_TSX: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../workspace-template/src/Composition.tsx"
    ));
    const ROOT_TSX: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../workspace-template/src/Root.tsx"
    ));

    let files: &[(&str, &str)] = &[
        ("index.tsx", INDEX_TSX),
        ("Preview.tsx", PREVIEW_TSX),
        ("Composition.tsx", COMPOSITION_TSX),
        ("Root.tsx", ROOT_TSX),
    ];

    for (name, content) in files {
        let dst = src_dir.join(name);
        if dst.exists() {
            continue;
        }
        std::fs::write(&dst, content).map_err(|e| {
            format!(
                "Failed to repair missing preview scaffold file {}: {}",
                dst.to_string_lossy(),
                e
            )
        })?;
    }

    Ok(())
}

pub struct PreviewServerState {
    pub process: Option<Child>,
    pub port: u16,
}

impl Default for PreviewServerState {
    fn default() -> Self {
        Self { process: None, port: 3030 }
    }
}

#[tauri::command]
pub fn start_preview_server(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, Mutex<PreviewServerState>>,
    workspace_path: String,
) -> Result<u16, String> {
    let mut server = state.lock().map_err(|e| e.to_string())?;
    
    if server.process.is_some() {
        println!("[Epris] Preview server already running on port {}", server.port);
        return Ok(server.port);
    }
    
    // Aggressive surgical cleanup for common ports
    utils::kill_process_on_port(3030);
    
    let port = utils::find_available_port(3030);
    println!("[Epris] Starting preview server in {} on port {}", workspace_path, port);

    let node_modules = std::path::Path::new(&workspace_path).join("node_modules");
    if !node_modules.exists() {
        return Err("Workspace dependencies are missing (node_modules not found). Run the First Run Wizard / pnpm install for this project first.".to_string());
    }

    // Repair missing scaffold files (e.g. src/index.tsx) so Vite can start.
    ensure_preview_scaffold(std::path::Path::new(&workspace_path))?;

    let logs_dir = std::path::Path::new(&workspace_path).join("logs");
    let _ = std::fs::create_dir_all(&logs_dir);
    let log_path = logs_dir.join("preview.log");
    let mut log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| format!("Failed to open preview.log: {}", e))?;
    let _ = writeln!(log_file, "\n--- Preview Server Log [{}] ---", chrono::Utc::now());

    // Pass args through to the underlying dev script (Vite expects `--port`).
    // Note: `pnpm run <script> --port <n>` forwards args to the script. (`--` is NOT stripped by pnpm.)
    let mut cmd = utils::create_shell_command("pnpm", &["run", "dev", "--port", &port.to_string()]);
    cmd.current_dir(&workspace_path);

    // Ensure the preview server can find our local pnpm (some machines won't have pnpm on PATH).
    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let toolchain_bin = app_dir.join("toolchain").join("bin");
    let path_sep = if cfg!(target_os = "windows") { ";" } else { ":" };
    let current_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}{}{}", toolchain_bin.to_string_lossy(), path_sep, current_path);
    cmd.env("PATH", new_path);
    // Reduce unreadable ANSI escape sequences in preview.log.
    cmd.env("NO_COLOR", "1");
    cmd.env("FORCE_COLOR", "0");
    cmd.env("TERM", "dumb");

    cmd.stdout(Stdio::from(log_file.try_clone().map_err(|e| e.to_string())?));
    cmd.stderr(Stdio::from(log_file));

    let mut child = cmd.spawn().map_err(|e| {
        let msg = format!("Failed to spawn preview server: {}", e);
        println!("[Epris] Error: {}", msg);
        msg
    })?;

    // If the child exits immediately, surface a helpful error.
    if let Ok(Some(status)) = child.try_wait() {
        return Err(format!(
            "Preview server exited immediately (code: {:?}). Check workspace/logs/preview.log",
            status.code()
        ));
    }
    
    server.process = Some(child);
    server.port = port;
    
    println!("[Epris] Preview server spawn attempt finished on port {}", port);
    Ok(port)
}

#[tauri::command]
pub fn stop_preview_server(state: tauri::State<'_, Mutex<PreviewServerState>>) -> Result<(), String> {
    let mut server = state.lock().map_err(|e| e.to_string())?;
    
    if let Some(mut child) = server.process.take() {
        println!("[Epris] Stopping preview server");
        utils::kill_process_tree(&mut child);
    }
    
    Ok(())
}
