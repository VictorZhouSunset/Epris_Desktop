use serde::{Serialize, Deserialize};
use std::path::Path;
use std::io::Write;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use crate::utils;
use tauri::Emitter;

static GATE_CANCEL_REQUESTED: AtomicBool = AtomicBool::new(false);
static GATE_CURRENT_PID: AtomicU32 = AtomicU32::new(0);

fn kill_pid_tree(pid: u32) {
    if pid == 0 {
        return;
    }
    println!("[Epris] Killing Gate process tree for PID: {}", pid);

    #[cfg(target_os = "windows")]
    {
        // On Windows, use taskkill to kill the entire process tree (/T)
        #[cfg(target_os = "windows")]
        use std::os::windows::process::CommandExt;

        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .status();
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Best-effort: send SIGKILL
        let _ = std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status();
    }
}

#[tauri::command]
pub fn stop_gate_validation() -> Result<(), String> {
    GATE_CANCEL_REQUESTED.store(true, Ordering::SeqCst);
    let pid = GATE_CURRENT_PID.swap(0, Ordering::SeqCst);
    if pid != 0 {
        kill_pid_tree(pid);
    }
    Ok(())
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum GateMode {
    Strict,
    Balanced,
    Fast,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GateResult {
    pub success: bool,
    pub passed: bool,
    pub typecheck_passed: bool,
    pub typecheck: bool,
    pub smoke_0_passed: bool,
    pub smoke_0: bool,
    pub smoke_mid_passed: bool,
    pub smoke_mid: bool,
    pub error_output: String,
    pub error: Option<String>,
    pub attempts: i32,
    // New duration fields
    pub duration_typecheck_sec: f64,
    pub duration_smoke_0_sec: f64,
    pub duration_smoke_mid_sec: f64,
    pub total_duration_sec: f64,
}

#[tauri::command]
pub async fn run_gate(workspace_path: String) -> Result<GateResult, String> {
    // Default command behavior stays strict.
    run_gate_with_mode(workspace_path, GateMode::Strict, None).await
}

pub async fn run_gate_with_mode(
    workspace_path: String,
    mode: GateMode,
    app_handle: Option<&tauri::AppHandle>,
) -> Result<GateResult, String> {
    // New run: clear any previous cancel request.
    GATE_CANCEL_REQUESTED.store(false, Ordering::SeqCst);
    GATE_CURRENT_PID.store(0, Ordering::SeqCst);

    println!("[Epris] Running Gate validation for {}", workspace_path);
    println!(
        "[Epris] Gate mode: {}",
        match mode {
            GateMode::Strict => "strict (typecheck + smoke:0 + smoke:mid)",
            GateMode::Balanced => "balanced (typecheck + smoke:mid)",
            GateMode::Fast => "fast (typecheck only)",
        }
    );
    let gate_start = std::time::Instant::now();
    
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
        duration_typecheck_sec: 0.0,
        duration_smoke_0_sec: 0.0,
        duration_smoke_mid_sec: 0.0,
        total_duration_sec: 0.0,
    };

    let canceled = || GATE_CANCEL_REQUESTED.load(Ordering::SeqCst);
    let set_progress = |percent: f64, step: &str| {
        if let Some(h) = app_handle {
            let _ = h.emit(
                "prompt-progress",
                serde_json::json!({ "percent": percent, "step": step }),
            );
        }
    };

    let run_step = |args: &[&str]| -> Result<std::process::Output, String> {
        if canceled() {
            return Err("Gate canceled".to_string());
        }

        let child = utils::create_shell_command("pnpm", args)
            .current_dir(&workspace_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn pnpm {:?}: {}", args, e))?;

        let pid = child.id();
        GATE_CURRENT_PID.store(pid, Ordering::SeqCst);

        let out = child
            .wait_with_output()
            .map_err(|e| format!("Failed to wait pnpm {:?}: {}", args, e))?;

        GATE_CURRENT_PID.store(0, Ordering::SeqCst);
        if canceled() {
            return Err("Gate canceled".to_string());
        }
        Ok(out)
    };
    
    // Step 1: Typecheck
    set_progress(96.0, "Gate: Typecheck");
    println!("[Epris] Gate Step 1/3: Running typecheck...");
    let typecheck_start = std::time::Instant::now();
    let typecheck_output = match run_step(&["run", "typecheck"]) {
        Ok(o) => o,
        Err(e) => {
            result.error_output = e.clone();
            result.error = Some(e);
            result.total_duration_sec = gate_start.elapsed().as_secs_f64();
            log_gate_result(&workspace_path, &result);
            return Ok(result);
        }
    };
    result.duration_typecheck_sec = typecheck_start.elapsed().as_secs_f64();
    println!("[Epris] Typecheck finished in {:.2}s", result.duration_typecheck_sec);
    
    if !typecheck_output.status.success() {
        let stderr = String::from_utf8_lossy(&typecheck_output.stderr);
        result.error_output = format!(
            "Typecheck failed:\n{}",
            stderr.chars().take(2000).collect::<String>()
        );
        result.error = Some(result.error_output.clone());
        result.total_duration_sec = gate_start.elapsed().as_secs_f64();
        log_gate_result(&workspace_path, &result);
        return Ok(result);
    }
    result.typecheck_passed = true;
    result.typecheck = true;

    if matches!(mode, GateMode::Fast) {
        result.success = true;
        result.passed = true;
        set_progress(100.0, "Gate: Passed");
        result.total_duration_sec = gate_start.elapsed().as_secs_f64();
        log_gate_result(&workspace_path, &result);
        return Ok(result);
    }

    // Strict mode: Smoke test frame 0 first (cheap sanity for CLI render)
    if matches!(mode, GateMode::Strict) {
        set_progress(97.0, "Gate: Smoke (frame 0)");
        println!("[Epris] Gate Step 2/3: Running smoke:0...");
        let smoke_0_start = std::time::Instant::now();
        let smoke_0_output = match run_step(&["run", "smoke:0"]) {
            Ok(o) => o,
            Err(e) => {
                result.error_output = e.clone();
                result.error = Some(e);
                result.total_duration_sec = gate_start.elapsed().as_secs_f64();
                log_gate_result(&workspace_path, &result);
                return Ok(result);
            }
        };
        result.duration_smoke_0_sec = smoke_0_start.elapsed().as_secs_f64();
        println!("[Epris] Smoke:0 finished in {:.2}s", result.duration_smoke_0_sec);

        if !smoke_0_output.status.success() {
            let stderr = String::from_utf8_lossy(&smoke_0_output.stderr);
            result.error_output = format!(
                "Smoke test (frame 0) failed:\n{}",
                stderr.chars().take(2000).collect::<String>()
            );
            result.error = Some(result.error_output.clone());
            result.total_duration_sec = gate_start.elapsed().as_secs_f64();
            log_gate_result(&workspace_path, &result);
            return Ok(result);
        }
        result.smoke_0_passed = true;
        result.smoke_0 = true;
    }

    // Balanced/Strict: Smoke test mid frame
    set_progress(99.0, "Gate: Smoke (mid)");
    println!("[Epris] Gate Step 3/3: Running smoke:mid...");
    let smoke_mid_start = std::time::Instant::now();
    let smoke_mid_output = match run_step(&["run", "smoke:mid"]) {
        Ok(o) => o,
        Err(e) => {
            result.error_output = e.clone();
            result.error = Some(e);
            result.total_duration_sec = gate_start.elapsed().as_secs_f64();
            log_gate_result(&workspace_path, &result);
            return Ok(result);
        }
    };
    result.duration_smoke_mid_sec = smoke_mid_start.elapsed().as_secs_f64();
    println!("[Epris] Smoke:mid finished in {:.2}s", result.duration_smoke_mid_sec);
    
    if !smoke_mid_output.status.success() {
        let stderr = String::from_utf8_lossy(&smoke_mid_output.stderr);
        result.error_output = format!(
            "Smoke test (mid frame) failed:\n{}",
            stderr.chars().take(2000).collect::<String>()
        );
        result.error = Some(result.error_output.clone());
        result.total_duration_sec = gate_start.elapsed().as_secs_f64();
        log_gate_result(&workspace_path, &result);
        return Ok(result);
    }
    result.smoke_mid_passed = true;
    result.smoke_mid = true;
    
    // All passed
    result.success = true;
    result.passed = true;
    set_progress(100.0, "Gate: Passed");
    
    println!("[Epris] Gate validation PASSED");
    result.total_duration_sec = gate_start.elapsed().as_secs_f64();
    log_gate_result(&workspace_path, &result);
    Ok(result)
}

pub fn log_gate_result(workspace_path: &str, result: &GateResult) {
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
        "duration_typecheck_sec": result.duration_typecheck_sec,
        "duration_smoke_0_sec": result.duration_smoke_0_sec,
        "duration_smoke_mid_sec": result.duration_smoke_mid_sec,
        "total_duration_sec": result.total_duration_sec,
    });
    
    if let Err(e) = writeln!(log_file, "{}", log_entry.to_string()) {
        eprintln!("[Epris] Failed to write to gate.jsonl: {}", e);
    }

    // Also log to performance.log for unified tracking
    let perf_log_path = Path::new(workspace_path).join("logs/performance.log");
    let perf_entry = format!(
        "[{}] TYPE: GATE | DURATION: {:.2}s | PASSED: {} | ATTEMPTS: {}\n",
        chrono::Utc::now().to_rfc3339(),
        result.total_duration_sec,
        result.passed,
        result.attempts
    );
    let _ = std::fs::OpenOptions::new().create(true).append(true).open(&perf_log_path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, perf_entry.as_bytes()));
}
