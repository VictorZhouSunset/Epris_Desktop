use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};
use std::process::Child;
use std::sync::Mutex;
use uuid::Uuid;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::io::Write;

const OPENCODE_SYSTEM_PROMPT: &str = "You are modifying a Remotion animation workspace. \
    Rules: \
    1. Keep composition id 'Main' unchanged. \
    2. Only edit files in src/ directory. \
    3. Ensure the composition still renders after changes. \
    4. Use Remotion's animation APIs (useCurrentFrame, interpolate, spring, etc.)";

pub mod state_manager;
pub mod environment;

use crate::state_manager::{StateManager, AppStateStore};
use crate::environment::{EnvironmentManager, EnvironmentStatus};
use tauri::Manager; // Ensure Manager is imported for path access if needed, though app_handle has path()


// ============================================================================
// Snapshot System - Data Structures
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SnapshotMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub timestamp: String,
    pub parent_id: Option<String>,
    pub is_manual: bool,
    pub prompt: Option<String>,
    pub session_id: String,
    pub gate_result: Option<GateResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotDAG {
    pub nodes: Vec<SnapshotMetadata>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
}

const SNAPSHOT_WHITELIST: &[&str] = &["src", "public"];
const FORBIDDEN_FILES: &[&str] = &[
    "package.json",
    "pnpm-lock.yaml",
    "tsconfig.json",
    "vite.config.ts",
    "index.html"
];

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub path: String,
    pub exists: bool,
}

/// Searches upward from the given path to find the project root containing the anchor.
fn find_project_root(start_path: &Path, anchor: &str) -> Option<PathBuf> {
    let mut current = start_path.to_path_buf();
    loop {
        if current.join(anchor).exists() {
            return Some(current);
        }
        if !current.pop() {
            break;
        }
    }
    None
}

fn is_port_available(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
}

fn find_available_port(start_port: u16) -> u16 {
    let mut port = start_port;
    while !is_port_available(port) && port < 65535 {
        port += 1;
    }
    port
}

fn kill_process_tree(child: &mut Child) {
    let pid = child.id();
    println!("[Epris] Killing process tree for PID: {}", pid);
    
    #[cfg(target_os = "windows")]
    {
        // On Windows, use taskkill to kill the entire process tree (/T)
        let _ = std::process::Command::new("taskkill")
            .args(&["/F", "/T", "/PID", &pid.to_string()])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .status();
    }
    
    // Fallback/Non-Windows kill
    let _ = child.kill();
}

fn create_shell_command(program: &str, args: &[&str]) -> std::process::Command {
    #[cfg(target_os = "windows")]
    {
        let mut cmd = std::process::Command::new("cmd");
        cmd.arg("/C");
        cmd.arg(program);
        cmd.args(args);
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        cmd
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut cmd = std::process::Command::new(program);
        cmd.args(args);
        cmd
    }
}

fn mirror_workspace(template_path: &Path, target_path: &Path) -> Result<(), String> {
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
        let status = create_shell_command("pnpm", &["install"])
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
fn get_workspace_config(_app_handle: tauri::AppHandle) -> Result<WorkspaceConfig, String> {
    #[cfg(debug_assertions)]
    let (template_path, target_path) = {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        println!("[Epris] Debug Mode - Manifest Dir: {}", manifest_dir);
        
        let project_root = find_project_root(Path::new(&manifest_dir), "workspace-template")
            .ok_or_else(|| format!("[Epris] Could not find 'workspace-template' anchor by searching upwards from {:?}", manifest_dir))?;

        println!("[Epris] Resolved Project Root via anchor: {:?}", project_root);
        (
            project_root.join("workspace-template"),
            project_root.join("workspace"),
        )
    };

    #[cfg(not(debug_assertions))]
    let (template_path, target_path) = {
        let resource_dir = _app_handle.path().resource_dir()
            .map_err(|e| format!("[Epris] Failed to get resource dir: {}", e))?;
// ...
        println!("[Epris] Production Mode - Resource Dir: {:?}", resource_dir);

        let template = app_handle.path().resolve("workspace-template", tauri::path::BaseDirectory::Resource)
            .map_err(|e| format!("[Epris] Failed to resolve resource 'workspace-template': {}", e))?;
        
        let target = app_handle.path().app_local_data_dir()
            .map_err(|e| format!("[Epris] Failed to get app data dir: {}", e))?
            .join("workspace");
            
        (template, target)
    };

    mirror_workspace(&template_path, &target_path)?;

    Ok(WorkspaceConfig {
        path: target_path.to_string_lossy().to_string(),
        exists: target_path.exists(),
    })
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// ============================================================================
// Preview Server Management
// ============================================================================

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
fn start_preview_server(
    state: tauri::State<'_, Mutex<PreviewServerState>>,
    workspace_path: String,
) -> Result<u16, String> {
    let mut server = state.lock().map_err(|e| e.to_string())?;
    
    if server.process.is_some() {
        println!("[Epris] Preview server already running on port {}", server.port);
        return Ok(server.port);
    }
    
    let port = find_available_port(3030);
    println!("[Epris] Starting preview server in {} on port {}", workspace_path, port);
    
    let child = create_shell_command("pnpm", &["run", "dev", "--port", &port.to_string()])
        .current_dir(&workspace_path)
        .spawn()
        .map_err(|e| {
            let msg = format!("Failed to start preview server: {}", e);
            println!("[Epris] Error: {}", msg);
            msg
        })?;
    
    server.process = Some(child);
    server.port = port;
    
    println!("[Epris] Preview server started successfully");
    Ok(port)
}

#[tauri::command]
fn stop_preview_server(state: tauri::State<'_, Mutex<PreviewServerState>>) -> Result<(), String> {
    let mut server = state.lock().map_err(|e| e.to_string())?;
    
    if let Some(mut child) = server.process.take() {
        println!("[Epris] Stopping preview server");
        kill_process_tree(&mut child);
    }
    
    Ok(())
}

// ============================================================================
// OpenCode Integration
// ============================================================================

pub struct OpenCodeState {
    pub process: Option<Child>,
    pub port: u16,
    pub session_id: Option<String>,
}

impl Default for OpenCodeState {
    fn default() -> Self {
        Self { process: None, port: 4096, session_id: None }
    }
}

#[tauri::command]
fn start_opencode(
    state: tauri::State<'_, Mutex<OpenCodeState>>,
    workspace_path: String,
) -> Result<u16, String> {
    let mut oc = state.lock().map_err(|e| e.to_string())?;
    
    if oc.process.is_some() {
        println!("[Epris] OpenCode server already running on port {}", oc.port);
        return Ok(oc.port);
    }
    
    let port = find_available_port(4096);
    println!("[Epris] Starting OpenCode server in {} on port {}", workspace_path, port);
    
    let child = create_shell_command("opencode", &["serve", "--hostname", "127.0.0.1", "--port", &port.to_string()])
        .current_dir(&workspace_path)
        .env("OPENCODE_PROVIDER", "opencode")
        .env("OPENCODE_MODEL", "opencode/minimax-m2.1-free")
        .spawn()
        .map_err(|e| {
            let msg = format!("Failed to start OpenCode: {}", e);
            println!("[Epris] Error: {}", msg);
            msg
        })?;
    
    oc.process = Some(child);
    oc.port = port;
    
    println!("[Epris] OpenCode server started successfully");
    Ok(port)
}

#[tauri::command]
fn stop_opencode(state: tauri::State<'_, Mutex<OpenCodeState>>) -> Result<(), String> {
    let mut oc = state.lock().map_err(|e| e.to_string())?;
    
    if let Some(mut child) = oc.process.take() {
        println!("[Epris] Stopping OpenCode server");
        kill_process_tree(&mut child);
    }
    oc.session_id = None;
    
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PromptResponse {
    pub success: bool,
    pub message: String,
    pub gate_result: Option<GateResult>,
    pub snapshot_id: Option<String>,        // Plan
    pub snapshot_timestamp: Option<String>, // Legacy
}

#[tauri::command]
async fn send_prompt(
    state: tauri::State<'_, Mutex<OpenCodeState>>,
    workspace_path: String,
    prompt: String,
) -> Result<PromptResponse, String> {
    let (port, session_id) = {
        let oc = state.lock().map_err(|e| e.to_string())?;
        (oc.port, oc.session_id.clone())
    };
    
    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{}", port);
    
    // System prompt for OpenCode
    let system_prompt = format!("{} workspace_path: {}", OPENCODE_SYSTEM_PROMPT, workspace_path);
    
    // Create or reuse session
    let sid = match session_id {
        Some(id) => {
            println!("[Epris] Reusing existing session: {}", id);
            id
        }
        None => {
            println!("[Epris] Creating new OpenCode session");
            let res = client.post(format!("{}/session", base_url))
                .json(&serde_json::json!({
                    "system": system_prompt
                }))
                .send()
                .await
                .map_err(|e| format!("Failed to create session: {}", e))?;
            
            let body: serde_json::Value = res.json().await.map_err(|e| format!("Failed to parse session response: {}", e))?;
            let new_id = body["id"].as_str()
                .ok_or("OpenCode response missing id")?
                .to_string();
            
            println!("[Epris] Created new session: {}", new_id);
            
            // Store session ID
            {
                let mut oc = state.lock().map_err(|e| e.to_string())?;
                oc.session_id = Some(new_id.clone());
            }
            new_id
        }
    };
    
    // Step 1: Create snapshot before modification
    println!("[Epris] Creating snapshot before modification");
    let snapshot_timestamp = create_snapshot(workspace_path.clone(), prompt.clone(), sid.clone())?;
    
    // Step 2: Send message to session
    println!("[Epris] Sending prompt to session: {}", sid);
    let res = client.post(format!("{}/session/{}/message", base_url, sid))
        .json(&serde_json::json!({
            "parts": [{ "type": "text", "text": prompt }]
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to send prompt: {}", e))?;
    
    let body: serde_json::Value = res.json().await.map_err(|e| format!("Failed to parse response JSON: {}", e))?;
    println!("[Epris] OpenCode response received");
    
    // Extract text from parts array
    let message_text = body["parts"]
        .as_array()
        .and_then(|parts| {
            parts.iter()
                .find(|p| p["type"] == "text")
                .and_then(|p| p["text"].as_str())
        })
        .unwrap_or("Check workspace for changes");

    // Step 3: Run Gate validation with auto-retry
    println!("[Epris] Running Gate validation with auto-retry");
    let gate_result = gate_loop(workspace_path.clone(), sid.clone(), 2, port).await?;
    
    // Step 4: Update snapshot with gate result
    update_snapshot_gate_result(workspace_path, snapshot_timestamp.clone(), gate_result.clone())?;

    Ok(PromptResponse {
        success: gate_result.success,
        message: message_text.to_string(),
        gate_result: Some(gate_result),
        snapshot_id: None, // Will be implemented in Task 5
        snapshot_timestamp: Some(snapshot_timestamp),
    })
}

#[tauri::command]
fn clear_session(state: tauri::State<'_, Mutex<OpenCodeState>>) -> Result<(), String> {
    let mut oc = state.lock().map_err(|e| e.to_string())?;
    println!("[Epris] Clearing session");
    oc.session_id = None;
    Ok(())
}

// ============================================================================
// Auto-Fix Loop
// ============================================================================

async fn request_fix(
    port: u16,
    session_id: String,
    gate_result: &GateResult,
) -> Result<(), String> {
    println!("[Epris] Requesting fix from OpenCode");
    
    let fix_prompt = format!(
        "The previous changes failed validation. Please fix ONLY the errors below. Do not refactor unrelated code.\n\nErrors:\n{}",
        gate_result.error_output
    );
    
    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{}", port);
    
    let res = client.post(format!("{}/session/{}/message", base_url, session_id))
        .json(&serde_json::json!({
            "parts": [{ "type": "text", "text": fix_prompt }]
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to send fix request: {}", e))?;
    
    let _body: serde_json::Value = res.json().await.map_err(|e| format!("Failed to parse fix response: {}", e))?;
    
    println!("[Epris] Fix request sent, OpenCode is working on it");
    Ok(())
}

async fn gate_loop(
    workspace_path: String,
    session_id: String,
    max_retries: i32,
    port: u16,
) -> Result<GateResult, String> {
    let mut attempts = 0;
    
    loop {
        attempts += 1;
        println!("[Epris] Gate attempt {}/{}", attempts, max_retries + 1);
        
        let mut result = run_gate(workspace_path.clone())?;
        result.attempts = attempts;
        
        if result.success {
            println!("[Epris] Gate passed on attempt {}", attempts);
            return Ok(result);
        }
        
        if attempts > max_retries {
            println!("[Epris] Gate failed after {} attempts", attempts);
            return Ok(result);
        }
        
        // Request fix and retry
        println!("[Epris] Gate failed, requesting fix (attempt {}/{})", attempts, max_retries);
        request_fix(port, session_id.clone(), &result).await?;
        
        // Wait for OpenCode to apply fixes (increased from 2s to 30s)
        println!("[Epris] Waiting 30s for OpenCode to complete fixes...");
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    }
}

// ============================================================================
// Gate Validation System
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GateResult {
    pub success: bool,           // Legacy
    pub passed: bool,            // Plan
    pub typecheck_passed: bool,  // Legacy
    pub typecheck: bool,         // Plan
    pub smoke_0_passed: bool,    // Legacy
    pub smoke_0: bool,           // Plan
    pub smoke_mid_passed: bool,  // Legacy
    pub smoke_mid: bool,         // Plan
    pub error_output: String,    // Legacy
    pub error: Option<String>,   // Plan
    pub attempts: i32,
}

#[tauri::command]
fn run_gate(workspace_path: String) -> Result<GateResult, String> {
    println!("[Epris] Running Gate validation on: {}", workspace_path);
    
    let mut result = GateResult {
        success: false,
        passed: false,
        typecheck_passed: false,
        typecheck: false,
        smoke_0_passed: false,
        smoke_0: false,
        smoke_mid_passed: false,
        smoke_mid: false,
        error_output: String::new(),
        error: None,
        attempts: 1,
    };
    
    // Step 1: Typecheck
    println!("[Epris] Gate Step 1/3: Running typecheck...");
    let typecheck_output = create_shell_command("pnpm", &["run", "typecheck"])
        .current_dir(&workspace_path)
        .output()
        .map_err(|e| format!("Failed to run typecheck: {}", e))?;
    
    if !typecheck_output.status.success() {
        let stderr = String::from_utf8_lossy(&typecheck_output.stderr);
        result.error = Some(result.error_output.clone());
        log_gate_result(&workspace_path, &result);
        return Ok(result);
    }
    result.typecheck_passed = true;
    result.typecheck = true;
    
    // Step 2: Smoke test frame 0
    println!("[Epris] Gate Step 2/3: Running smoke:0...");
    let smoke_0_output = create_shell_command("pnpm", &["run", "smoke:0"])
        .current_dir(&workspace_path)
        .output()
        .map_err(|e| format!("Failed to run smoke:0: {}", e))?;
    
    if !smoke_0_output.status.success() {
        let stderr = String::from_utf8_lossy(&smoke_0_output.stderr);
        result.error = Some(result.error_output.clone());
        log_gate_result(&workspace_path, &result);
        return Ok(result);
    }
    result.smoke_0_passed = true;
    result.smoke_0 = true;
    
    // Step 3: Smoke test mid frame
    println!("[Epris] Gate Step 3/3: Running smoke:mid...");
    let smoke_mid_output = create_shell_command("pnpm", &["run", "smoke:mid"])
        .current_dir(&workspace_path)
        .output()
        .map_err(|e| format!("Failed to run smoke:mid: {}", e))?;
    
    if !smoke_mid_output.status.success() {
        let stderr = String::from_utf8_lossy(&smoke_mid_output.stderr);
        result.error = Some(result.error_output.clone());
        log_gate_result(&workspace_path, &result);
        return Ok(result);
    }
    result.smoke_mid_passed = true;
    result.smoke_mid = true;
    
    // All passed
    result.success = true;
    result.passed = true;
    result.typecheck = true;
    result.smoke_0 = true;
    result.smoke_mid = true;
    println!("[Epris] Gate validation PASSED");
    log_gate_result(&workspace_path, &result);
    Ok(result)
}

fn log_gate_result(workspace_path: &str, result: &GateResult) {
    let log_path = Path::new(workspace_path).join("logs").join("gate.jsonl");
    
    // Ensure logs directory exists
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    
    let mut log_file = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[Epris] Failed to open gate.jsonl: {}", e);
            return;
        }
    };
    
    let log_entry = serde_json::json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "success": result.success,
        "typecheck_passed": result.typecheck_passed,
        "smoke_0_passed": result.smoke_0_passed,
        "smoke_mid_passed": result.smoke_mid_passed,
        "error_output": result.error_output,
        "attempts": result.attempts,
        "workspace_path": workspace_path,
    });
    
    if let Err(e) = writeln!(log_file, "{}", log_entry.to_string()) {
        eprintln!("[Epris] Failed to write to gate.jsonl: {}", e);
    }
}

// ============================================================================
// Snapshot System
// ============================================================================

const LEGACY_SNAPSHOT_WHITELIST: &[&str] = &[
    "src",
    "public",
    "index.html",
    "package.json",
    "pnpm-lock.yaml",
    "tsconfig.json",
    "vite.config.ts",
];

fn is_whitelisted(path: &Path) -> bool {
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    
    // Check exact matches
    for &pattern in LEGACY_SNAPSHOT_WHITELIST {
        if file_name == pattern {
            return true;
        }
        // Check tsconfig*.json pattern
        if pattern == "tsconfig.json" && file_name.starts_with("tsconfig") && file_name.ends_with(".json") {
            return true;
        }
        // Check vite.config.* pattern
        if pattern == "vite.config.ts" && file_name.starts_with("vite.config.") {
            return true;
        }
    }
    false
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    if !src.exists() {
        return Ok(()); // Skip if doesn't exist
    }
    
    std::fs::create_dir_all(dst)
        .map_err(|e| format!("Failed to create dir {}: {}", dst.display(), e))?;
    
    for entry in std::fs::read_dir(src)
        .map_err(|e| format!("Failed to read dir {}: {}", src.display(), e))? 
    {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let file_name = path.file_name().ok_or("No filename")?;
        let dest_path = dst.join(file_name);
        
        // Skip excluded directories
        if path.is_dir() {
            let name = file_name.to_str().unwrap_or("");
            if name == "node_modules" || name == ".epris" || name == "out" || name == "dist" {
                continue;
            }
            copy_dir_recursive(&path, &dest_path)?;
        } else if path.is_file() {
            std::fs::copy(&path, &dest_path)
                .map_err(|e| format!("Failed to copy {}: {}", path.display(), e))?;
        }
    }
    Ok(())
}

fn estimate_dir_size(path: &Path) -> Result<u64, std::io::Error> {
    let mut total_size = 0u64;
    
    if path.is_file() {
        return Ok(path.metadata()?.len());
    }
    
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();
            
            if entry_path.is_dir() {
                total_size += estimate_dir_size(&entry_path)?;
            } else {
                total_size += entry.metadata()?.len();
            }
        }
    }
    
    Ok(total_size)
}

fn cleanup_old_snapshots(history_dir: &Path, keep_count: usize) -> Result<(), String> {
    if !history_dir.exists() {
        return Ok(());
    }
    
    let mut snapshots: Vec<_> = std::fs::read_dir(history_dir)
        .map_err(|e| format!("Failed to read history dir: {}", e))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    
    // Sort by directory name (timestamp)
    snapshots.sort_by_key(|e| e.file_name());
    
    // Delete oldest snapshots if we exceed keep_count
    if snapshots.len() > keep_count {
        let to_delete = snapshots.len() - keep_count;
        for snapshot in snapshots.iter().take(to_delete) {
            let path = snapshot.path();
            println!("[Epris] Deleting old snapshot: {:?}", path);
            if let Err(e) = std::fs::remove_dir_all(&path) {
                eprintln!("[Epris] Failed to delete snapshot {:?}: {}", path, e);
                // Continue even if deletion fails
            }
        }
    }
    
    Ok(())
}

#[tauri::command]
fn create_snapshot(
    workspace_path: String,
    prompt: String,
    session_id: String,
) -> Result<String, String> {
    let workspace = Path::new(&workspace_path);
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let snapshot_dir = workspace.join(".epris").join("history").join(&timestamp);
    
    println!("[Epris] Creating snapshot at: {:?}", snapshot_dir);
    
    // Check disk space before snapshot (require at least 100MB free)
    if let Ok(metadata) = std::fs::metadata(&workspace) {
        // Estimate src/ size as proxy for snapshot size
        if let Ok(src_size) = estimate_dir_size(&workspace.join("src")) {
            // Require 2x the estimated size plus 100MB buffer
            let required_space = src_size * 2 + 100_000_000;
            // Note: fs2 crate needed for portable disk space check, for V0 we'll skip
            // TODO: Add proper disk space check with fs2 crate
            println!("[Epris] Estimated snapshot size: {} bytes", src_size);
        }
    }
    
    std::fs::create_dir_all(&snapshot_dir)
        .map_err(|e| format!("Failed to create snapshot dir: {}", e))?;
    
    // Cleanup old snapshots (keep last 10)
    cleanup_old_snapshots(&workspace.join(".epris").join("history"), 10)?;
    
    // Copy whitelisted files/dirs
    for &item in LEGACY_SNAPSHOT_WHITELIST {
        let src_path = workspace.join(item);
        let dst_path = snapshot_dir.join(item);
        
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if src_path.is_file() {
            if let Some(parent) = dst_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create parent dir: {}", e))?;
            }
            std::fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("Failed to copy {}: {}", item, e))?;
        }
    }
    
    // Write meta.json
    let meta = serde_json::json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "prompt": prompt,
        "session_id": session_id,
        "gate_result": null,
        "files": SNAPSHOT_WHITELIST,
    });
    
    let meta_path = snapshot_dir.join("meta.json");
    std::fs::write(&meta_path, serde_json::to_string_pretty(&meta).unwrap())
        .map_err(|e| format!("Failed to write meta.json: {}", e))?;
    
    println!("[Epris] Snapshot created: {}", timestamp);
    Ok(timestamp)
}

#[tauri::command]
fn rollback_snapshot(workspace_path: String) -> Result<String, String> {
    let workspace = Path::new(&workspace_path);
    let history_dir = workspace.join(".epris").join("history");
    
    if !history_dir.exists() {
        return Err("No snapshots found".to_string());
    }
    
    // Find latest snapshot
    let mut snapshots: Vec<_> = std::fs::read_dir(&history_dir)
        .map_err(|e| format!("Failed to read history dir: {}", e))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    
    snapshots.sort_by_key(|e| e.file_name());
    
    let latest = snapshots.last()
        .ok_or("No snapshots found")?;
    
    let snapshot_path = latest.path();
    let snapshot_name = latest.file_name().to_string_lossy().to_string();
    
    println!("[Epris] Rolling back to snapshot: {}", snapshot_name);
    
    // Restore whitelisted files/dirs
    for &item in LEGACY_SNAPSHOT_WHITELIST {
        let src_path = snapshot_path.join(item);
        let dst_path = workspace.join(item);
        
        if !src_path.exists() {
            continue;
        }
        
        // Remove existing first
        if dst_path.exists() {
            if dst_path.is_dir() {
                std::fs::remove_dir_all(&dst_path)
                    .map_err(|e| format!("Failed to remove old {}: {}", item, e))?;
            } else {
                std::fs::remove_file(&dst_path)
                    .map_err(|e| format!("Failed to remove old {}: {}", item, e))?;
            }
        }
        
        // Copy from snapshot
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("Failed to restore {}: {}", item, e))?;
        }
    }
    
    println!("[Epris] Rollback complete");
    Ok(snapshot_name)
}

#[tauri::command]
fn update_snapshot_gate_result(
    workspace_path: String,
    snapshot_timestamp: String,
    gate_result: GateResult,
) -> Result<(), String> {
    let meta_path = Path::new(&workspace_path)
        .join(".epris")
        .join("history")
        .join(snapshot_timestamp)
        .join("meta.json");
    
    if !meta_path.exists() {
        return Err("Snapshot meta.json not found".to_string());
    }
    
    let mut meta: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&meta_path)
            .map_err(|e| format!("Failed to read meta.json: {}", e))?
    ).map_err(|e| format!("Failed to parse meta.json: {}", e))?;
    
    meta["gate_result"] = serde_json::to_value(&gate_result)
        .map_err(|e| format!("Failed to serialize gate_result: {}", e))?;
    
    std::fs::write(&meta_path, serde_json::to_string_pretty(&meta).unwrap())
        .map_err(|e| format!("Failed to write meta.json: {}", e))?;
    
    Ok(())
}

// ============================================================================
// Export System
// ============================================================================

pub struct ExportState {
    pub is_exporting: bool,
}

impl Default for ExportState {
    fn default() -> Self {
        Self { is_exporting: false }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportResult {
    pub success: bool,
    pub output_path: Option<String>,
    pub error: Option<String>,
}

#[tauri::command]
async fn export_video(
    state: tauri::State<'_, Mutex<ExportState>>,
    workspace_path: String,
) -> Result<ExportResult, String> {
    // Check if already exporting
    {
        let export_state = state.lock().map_err(|e| e.to_string())?;
        if export_state.is_exporting {
            return Err("Export already in progress".to_string());
        }
    }
    
    // Set exporting flag
    {
        let mut export_state = state.lock().map_err(|e| e.to_string())?;
        export_state.is_exporting = true;
    }
    
    // Execute export with guaranteed flag reset
    let result = async {
        println!("[Epris] Starting video export");
        
        let workspace = Path::new(&workspace_path);
        let out_dir = workspace.join("out");
        let output_path = out_dir.join("video.mp4");
        let log_path = workspace.join("logs").join("export.log");
        
        // Ensure out directory exists
        std::fs::create_dir_all(&out_dir)
            .map_err(|e| format!("Failed to create out dir: {}", e))?;
        
        // Ensure logs directory exists
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create logs dir: {}", e))?;
        }
        
        // Open log file
        let mut log_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|e| format!("Failed to open export.log: {}", e))?;
        
        writeln!(log_file, "\n--- Export Log [{:?}] ---", chrono::Utc::now())
            .map_err(|e| e.to_string())?;
        
        // Run export command
        let export_output = create_shell_command("pnpm", &["run", "export"])
            .current_dir(&workspace_path)
            .output()
            .map_err(|e| {
                let msg = format!("Failed to run export: {}", e);
                let _ = writeln!(log_file, "{}", msg);
                msg
            })?;
        
        // Write output to log
        writeln!(log_file, "stdout:\n{}", String::from_utf8_lossy(&export_output.stdout))
            .map_err(|e| e.to_string())?;
        writeln!(log_file, "stderr:\n{}", String::from_utf8_lossy(&export_output.stderr))
            .map_err(|e| e.to_string())?;
        
        if !export_output.status.success() {
            let error_msg = format!(
                "Export failed with exit code: {:?}\nCheck workspace/logs/export.log for details",
                export_output.status.code()
            );
            println!("[Epris] {}", error_msg);
            return Ok(ExportResult {
                success: false,
                output_path: None,
                error: Some(error_msg),
            });
        }
        
        println!("[Epris] Export completed successfully: {:?}", output_path);
        Ok(ExportResult {
            success: true,
            output_path: Some(output_path.to_string_lossy().to_string()),
            error: None,
        })
    }.await;
    
    // Always reset exporting flag before returning
    {
        let mut export_state = state.lock().map_err(|e| e.to_string())?;
        export_state.is_exporting = false;
    }
    
    result
}

#[tauri::command]
fn get_environment_status(app_handle: tauri::AppHandle, provider: String) -> Result<EnvironmentStatus, String> {
    let app_dir = app_handle.path().app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let env_manager = EnvironmentManager::new(app_dir);
    Ok(env_manager.check_environment(&provider))
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
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(PreviewServerState::default()))
        .manage(Mutex::new(OpenCodeState::default()))
        .manage(Mutex::new(ExportState::default()))
        .invoke_handler(tauri::generate_handler![
            greet, 
            get_workspace_config,
            start_preview_server,
            stop_preview_server,
            start_opencode,
            stop_opencode,
            send_prompt,
            clear_session,
            run_gate,
            create_snapshot,
            rollback_snapshot,
            update_snapshot_gate_result,
            update_snapshot_gate_result,
            export_video,
            get_environment_status,
            get_app_state
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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

        mirror_workspace(&template_path, &target_path).expect("Should succeed");
        
        assert!(target_path.exists());
        assert!(target_path.join("dummy.txt").exists());
        assert!(target_path.join("logs").exists());
    }
}
