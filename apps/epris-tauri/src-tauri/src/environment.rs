use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use crate::state_manager::{ToolchainState, ToolchainVersion};
use tauri::{Manager, Emitter};
use walkdir::WalkDir;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use crate::toolchain;

#[derive(Debug, Serialize, Deserialize)]
pub struct EnvironmentStatus {
    pub node_valid: bool,
    pub pnpm_valid: bool,
    pub provider_cli_valid: bool,
    pub workspace_deps_valid: bool,
    pub skills_valid: bool,
    pub missing: Vec<String>,
    pub details: ToolchainState,
}

pub struct EnvironmentManager {
    pub toolchain_dir: PathBuf,
}

impl EnvironmentManager {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self {
            toolchain_dir: app_data_dir.join("toolchain"),
        }
    }

    pub fn get_bin_dir(&self) -> PathBuf {
        self.toolchain_dir.join("bin")
    }

    pub fn check_environment(&self, provider: &str, workspace_path: Option<&str>) -> EnvironmentStatus {
        let mut status = EnvironmentStatus {
            node_valid: false,
            pnpm_valid: false,
            provider_cli_valid: false,
            workspace_deps_valid: false,
            skills_valid: false,
            missing: Vec::new(),
            details: ToolchainState::default(),
        };
 
        // Check Node
        match self.check_command("node", &["--version"]) {
            Ok(v) => {
                status.node_valid = true;
                // Determine source (heuristic)
                let source = if self.is_local("node.exe") { "local" } else { "system" };
                status.details.node = Some(ToolchainVersion { version: v, source: source.into() });
            },
            Err(_) => status.missing.push("node".into()),
        }
 
        // Check pnpm
        match self.check_command("pnpm", &["--version"]) {
            Ok(v) => {
                status.pnpm_valid = true;
                let source = if self.is_local("pnpm") || self.is_local("pnpm.cmd") { "local" } else { "system" };
                status.details.pnpm = Some(ToolchainVersion { version: v, source: source.into() });
            },
            Err(_) => status.missing.push("pnpm".into()),
        }
 
        // Check Provider (opencode or gemini)
        let cmd = if provider == "gemini" { "gemini" } else { "opencode" };
        match self.check_command_any(cmd, &[&["--version"], &["version"]]) {
            Ok(v) => {
                status.provider_cli_valid = true;
                let source = self.detect_source_from_where(cmd).unwrap_or_else(|| {
                    if self.is_local(cmd)
                        || self.is_local(&format!("{}.cmd", cmd))
                        || self.is_local(&format!("{}.exe", cmd))
                    {
                        "local".to_string()
                    } else {
                        "system".to_string()
                    }
                });
                status.details.provider_cli = Some(ToolchainVersion {
                    version: v,
                    source: source.into(),
                });
            }
            Err(_) => {
                // Some CLIs may not support `--version` but still exist on PATH.
                if self.command_exists(cmd).is_some() {
                    status.provider_cli_valid = true;
                    let source = self.detect_source_from_where(cmd).unwrap_or_else(|| "system".to_string());
                    status.details.provider_cli = Some(ToolchainVersion {
                        version: "unknown".into(),
                        source: source.into(),
                    });
                } else {
                    status.missing.push(cmd.into());
                }
            }
        };
 
        // Check Workspace Deps
        if let Some(path) = workspace_path {
            let node_modules = std::path::Path::new(path).join("node_modules");
            if node_modules.exists() {
                status.workspace_deps_valid = true;
            } else {
                status.missing.push("workspace_deps".into());
            }
 
            // Check Skills: any <provider>/.*/skills/**/SKILL.md
            let gemini_skills_dir = std::path::Path::new(path).join(".gemini").join("skills");
            let opencode_skills_dir = std::path::Path::new(path).join(".opencode").join("skills");

            let g_exists = has_any_skill_md(&gemini_skills_dir);
            let o_exists = has_any_skill_md(&opencode_skills_dir);

            // Step-11 policy: install projections for both providers
            if g_exists && o_exists {
                status.skills_valid = true;
            } else {
                println!("[Epris] Missing skills:");
                println!("  Gemini dir: {:?} (Has SKILL.md: {})", gemini_skills_dir, g_exists);
                println!("  OpenCode dir: {:?} (Has SKILL.md: {})", opencode_skills_dir, o_exists);
                status.missing.push("skills".into());
            }
        }
 
        status
    }
    
    fn is_local(&self, binary_name: &str) -> bool {
        self.get_bin_dir().join(binary_name).exists()
    }

    fn check_command_any(&self, program: &str, candidates: &[&[&str]]) -> Result<String, String> {
        let mut last_err = None;
        for args in candidates {
            match self.check_command(program, args) {
                Ok(v) if !v.trim().is_empty() => return Ok(v),
                Ok(_) => return Ok(String::new()),
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap_or_else(|| "Command failed".into()))
    }

    #[cfg(target_os = "windows")]
    fn command_exists(&self, program: &str) -> Option<String> {
        let bin_dir = self.get_bin_dir();
        let path_env = std::env::var("PATH").unwrap_or_default();
        let new_path = format!("{};{}", bin_dir.to_string_lossy(), path_env);

        let mut cmd = Command::new("cmd");
        cmd.arg("/C").arg("where").arg(program);
        cmd.env("PATH", new_path);
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

        let out = cmd.output().ok()?;
        if !out.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&out.stdout);
        stdout.lines().next().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
    }

    #[cfg(not(target_os = "windows"))]
    fn command_exists(&self, _program: &str) -> Option<String> {
        None
    }

    #[cfg(target_os = "windows")]
    fn detect_source_from_where(&self, program: &str) -> Option<String> {
        let Some(p) = self.command_exists(program) else {
            return None;
        };
        let bin_dir = self.get_bin_dir().to_string_lossy().to_string().to_ascii_lowercase();
        let p_norm = p.to_ascii_lowercase();
        if p_norm.starts_with(&bin_dir) {
            return Some("local".to_string());
        }
        Some("system".to_string())
    }

    #[cfg(not(target_os = "windows"))]
    fn detect_source_from_where(&self, _program: &str) -> Option<String> {
        None
    }

    fn check_command(&self, program: &str, args: &[&str]) -> Result<String, String> {
        let bin_dir = self.get_bin_dir();
        
        let path_env = std::env::var("PATH").unwrap_or_default();
        let new_path = format!("{};{}", bin_dir.to_string_lossy(), path_env);

        #[cfg(target_os = "windows")]
        let mut cmd = Command::new("cmd");
        #[cfg(target_os = "windows")]
        {
            cmd.arg("/C").arg(program).args(args);
            cmd.env("PATH", new_path);
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        
        // Fallback for non-windows (though user is windows)
        #[cfg(not(target_os = "windows"))]
        let mut cmd = Command::new(program);
        #[cfg(not(target_os = "windows"))]
        {
            cmd.args(args);
             // TODO: Set path for unix
        }

        let output = cmd.output().map_err(|e| e.to_string())?;
        
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(stdout)
        } else {
            Err("Command failed".into())
        }
    }
}

fn has_any_skill_md(dir: &std::path::Path) -> bool {
    if !dir.exists() {
        return false;
    }
    WalkDir::new(dir)
        .max_depth(3)
        .into_iter()
        .filter_map(Result::ok)
        .any(|e| e.file_type().is_file() && e.file_name().to_string_lossy().eq_ignore_ascii_case("SKILL.md"))
}


#[tauri::command]
pub fn get_gemini_auth_status(app_handle: tauri::AppHandle) -> Result<bool, String> {
    // Check for OAuth credentials in ~/.gemini/oauth_creds.json
    let home_dir = app_handle.path().home_dir()
        .map_err(|e| format!("Failed to get home dir: {}", e))?;
    let oauth_creds = home_dir.join(".gemini").join("oauth_creds.json");
    
    if oauth_creds.exists() {
        return Ok(true);
    }

    // Fallback: Check for API key in state.json
    let app_dir = app_handle.path().app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let state_manager = crate::state_manager::StateManager::new(app_dir);
    Ok(state_manager.read().gemini_api_key.is_some())
}

#[tauri::command]
pub fn open_gemini_login(_app_handle: tauri::AppHandle) -> Result<(), String> {
    let _ = open::that("https://aistudio.google.com/app/apikey");
    Ok(())
}

#[tauri::command]
pub fn set_gemini_api_key(app_handle: tauri::AppHandle, key: String) -> Result<(), String> {
    let app_dir = app_handle.path().app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let state_manager = crate::state_manager::StateManager::new(app_dir);
    let mut state = state_manager.read();
    state.gemini_api_key = Some(key);
    state_manager.write(&state).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn install_missing_dependencies(
    window: tauri::Window,
    workspace_path: String,
    provider: String
) -> Result<(), String> {
    println!("[Epris] Installing dependencies for {} in {}...", provider, workspace_path);
    
    // Simulate stages as per spec
    let stages = [
        ("Detecting environment", 10),
        ("Installing toolchain", 30),
        ("Bootstrapping workspace", 60),
        ("Installing Remotion skills", 85),
        ("Finalizing", 100),
    ];

    for (step, percent) in stages {
        let _ = window.emit("env_install_progress", serde_json::json!({
            "step": step,
            "percent": percent
        }));
        let _ = window.emit("env_install_log", format!("Stage: {}", step));

        if step == "Bootstrapping workspace" {
            let app_dir = window
                .app_handle()
                .path()
                .app_local_data_dir()
                .map_err(|e| e.to_string())?;
            let env_manager = EnvironmentManager::new(app_dir);
            run_pnpm_install(&window, &workspace_path, &env_manager).await?;
        }

        if step == "Installing toolchain" {
            toolchain::ensure_local_toolchain(Some(&window), &provider, &window.app_handle()).await?;
        }

        if step == "Installing Remotion skills" {
             let app_dir = window.app_handle().path().app_local_data_dir().map_err(|e| e.to_string())?;
             let env_manager = EnvironmentManager::new(app_dir);
             crate::skills::SkillsManager::install_remotion_skills(Some(&window), &workspace_path, &provider, &env_manager).await?;
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    }

    Ok(())
}

async fn run_pnpm_install(
    window: &tauri::Window,
    workspace_path: &str,
    env_manager: &EnvironmentManager,
) -> Result<(), String> {
    let _ = window.emit("env_install_log", "Running pnpm install...".to_string());

    let bin_dir = env_manager.get_bin_dir();
    let path_env = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{};{}", bin_dir.to_string_lossy(), path_env);

    let mut child = crate::utils::create_async_shell_command(
        "pnpm",
        &["install", "--frozen-lockfile", "--prefer-offline"],
    )
        .current_dir(workspace_path)
        .env("PATH", new_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn pnpm install: {}", e))?;

    let stdout = child.stdout.take().ok_or("Failed to capture pnpm stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture pnpm stderr")?;
    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    let w1 = window.clone();
    let stdout_task = tokio::spawn(async move {
        while let Ok(Some(line)) = stdout_reader.next_line().await {
            let _ = w1.emit("env_install_log", line);
        }
    });

    let w2 = window.clone();
    let stderr_task = tokio::spawn(async move {
        while let Ok(Some(line)) = stderr_reader.next_line().await {
            let _ = w2.emit("env_install_log", line);
        }
    });

    let status = child.wait().await.map_err(|e| e.to_string())?;
    let _ = stdout_task.await;
    let _ = stderr_task.await;

    if !status.success() {
        return Err(format!("pnpm install failed (exit code: {:?})", status.code()));
    }

    let _ = window.emit("env_install_log", "pnpm install completed.".to_string());
    Ok(())
}

#[tauri::command]
pub fn open_gemini_auth_terminal(_app_handle: tauri::AppHandle) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        // Spawns a new visible Command Prompt window running the command
        // User said: directly "gemini" can be, first time gemini will automatically auth
        let app_dir = _app_handle.path().app_local_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {}", e))?;
        let env_manager = EnvironmentManager::new(app_dir);
        let bin_dir = env_manager.get_bin_dir();
        let path_env = std::env::var("PATH").unwrap_or_default();
        let new_path = format!("{};{}", bin_dir.to_string_lossy(), path_env);

        std::process::Command::new("cmd")
            .args(["/C", "start", "cmd", "/k", "gemini"])
            .env("PATH", new_path)
            .spawn()
            .map_err(|e| format!("Failed to open terminal: {}", e))?;
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        // Try standard terminal emulators for macOS/Linux (basic best effort)
        // For macOS: open -a Terminal
        // For Linux: x-terminal-emulator
        // Keep it simple for now as user is on Windows
        return Err("Not implemented for non-Windows yet".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_environment_manager_paths() {
        let tmp_path = std::path::PathBuf::from(r"C:\tmp\epris_test"); // Mock path
        let env = EnvironmentManager::new(tmp_path.clone());
        assert_eq!(env.toolchain_dir, tmp_path.join("toolchain"));
        assert_eq!(env.get_bin_dir(), tmp_path.join("toolchain").join("bin"));
    }
}
