use serde::{Serialize, Deserialize};
use std::path::Path;
use std::io::Write;
use std::sync::Mutex;
use crate::utils;
use tauri::Emitter;
use tauri::Manager;
use tokio::io::{AsyncBufReadExt, BufReader};
use std::process::Stdio;

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

#[derive(Clone, Serialize)]
struct ExportProgress {
    percent: f64,
}

#[tauri::command]
pub async fn export_video(
    app_handle: tauri::AppHandle,
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
        let start_time = std::time::Instant::now();
        
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
        
        // Initialize progress at 0
        let _ = app_handle.emit("export-progress", ExportProgress { percent: 0.0 });
        
        // Run export command asynchronously
        let mut child = utils::create_async_shell_command("pnpm", &["run", "export"])
            .current_dir(&workspace_path)
            .env(
                "PATH",
                {
                    let app_dir = app_handle
                        .path()
                        .app_local_data_dir()
                        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
                    let toolchain_bin = app_dir.join("toolchain").join("bin");
                    let sep = if cfg!(target_os = "windows") { ";" } else { ":" };
                    let current = std::env::var("PATH").unwrap_or_default();
                    format!("{}{}{}", toolchain_bin.to_string_lossy(), sep, current)
                },
            )
            .env("NO_COLOR", "1")
            .env("FORCE_COLOR", "0")
            .env("TERM", "dumb")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn export: {}", e))?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        
        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        let app_clone = app_handle.clone();
        
        // Handle stdout for progress parsing
        let stdout_task = tokio::spawn(async move {
            let mut captured_stdout = String::new();
            while let Ok(Some(line)) = stdout_reader.next_line().await {
                // Parse progress
                // Remotion output examples:
                // - "Bundling..." -> 5%
                // - "Rendering: 15% [=======...]" -> 5% + (0.95 * 15)
                
                if line.contains("Bundling") {
                    let _ = app_clone.emit("export-progress", ExportProgress { percent: 5.0 });
                } else if line.contains("Rendering:") {
                    if let Some(pct_part) = line.split("Rendering:").last() {
                        // Extract first number-like string (robust to colors/ansi)
                        let pct_text = pct_part.trim()
                            .chars()
                            .take_while(|c| c.is_digit(10) || *c == '.')
                            .collect::<String>();
                        
                        if !pct_text.is_empty() {
                            if let Ok(p) = pct_text.parse::<f64>() {
                                // Scale so Bundling is 5% and Rendering is the remaining 95%
                                let scaled_p = 5.0 + (p * 0.95);
                                let _ = app_clone.emit("export-progress", ExportProgress { percent: scaled_p });
                            }
                        }
                    }
                }
                
                captured_stdout.push_str(&line);
                captured_stdout.push('\n');
            }
            captured_stdout
        });

        let stderr_task = tokio::spawn(async move {
            let mut captured_stderr = String::new();
            while let Ok(Some(line)) = stderr_reader.next_line().await {
                captured_stderr.push_str(&line);
                captured_stderr.push('\n');
            }
            captured_stderr
        });

        let status = child.wait().await.map_err(|e| e.to_string())?;
        let captured_stdout = stdout_task.await.unwrap_or_default();
        let captured_stderr = stderr_task.await.unwrap_or_default();

        // Write output to log
        writeln!(log_file, "stdout:\n{}", captured_stdout).map_err(|e| e.to_string())?;
        writeln!(log_file, "stderr:\n{}", captured_stderr).map_err(|e| e.to_string())?;
        
        if !status.success() {
            let error_msg = format!(
                "Export failed with exit code: {:?}\nCheck workspace/logs/export.log for details",
                status.code()
            );
            println!("[Epris] {}", error_msg);
            let _ = writeln!(log_file, "Export failed in {:.2}s", start_time.elapsed().as_secs_f64());
            return Ok(ExportResult {
                success: false,
                output_path: None,
                error: Some(error_msg),
            });
        }
        
        println!("[Epris] Export completed successfully: {:?}", output_path);
        let duration_sec = start_time.elapsed().as_secs_f64();
        let _ = writeln!(log_file, "Export finished in {:.2}s", duration_sec);
        
        // Also log to performance.log for unified tracking
        let log_dir = Path::new(&workspace_path).join("logs");
        let perf_log_path = log_dir.join("performance.log");
        let perf_entry = format!(
            "[{}] TYPE: EXPORT | DURATION: {:.2}s | PATH: {:?}\n",
            chrono::Utc::now().to_rfc3339(),
            duration_sec,
            output_path
        );
        let _ = std::fs::OpenOptions::new().create(true).append(true).open(&perf_log_path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, perf_entry.as_bytes()));

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
