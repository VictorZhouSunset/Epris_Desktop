use std::process::Child;
use std::sync::Mutex;
use crate::utils;

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
    
    let child = utils::create_shell_command("pnpm", &["run", "dev", "--port", &port.to_string()])
        .current_dir(&workspace_path)
        .spawn()
        .map_err(|e| {
            let msg = format!("Failed to spawn preview server: {}", e);
            println!("[Epris] Error: {}", msg);
            msg
        })?;
    
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
