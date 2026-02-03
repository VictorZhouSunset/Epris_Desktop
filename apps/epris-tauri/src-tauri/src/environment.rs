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
use std::io::Write;
use crate::install_log;
use crate::projects;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

const BASELINE_NPM_PACKAGES: &[&str] = &[
    // Validation / determinism
    "zod",
    "seedrandom",
    // LaTeX / math rendering
    "katex",
    "react-katex",
    // Easing / curves
    "d3-ease",
    "bezier-easing",
    // Noise / natural motion
    "simplex-noise",
    // Color utilities
    "culori",
    // SVG path + morph
    "svg-path-properties",
    "flubber",
    // Charts (D3 route)
    "d3-scale",
    "d3-shape",
    "d3-array",
    "d3-interpolate",
];

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BaselinePackagesInfo {
    pub signature: String,
    pub missing_in_workspace: Vec<String>,
    pub missing_in_template: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BaselinePackagesInstallResult {
    pub signature: String,
    pub installed_in_workspace: Vec<String>,
    pub installed_in_template: Vec<String>,
}

pub struct EnvironmentManager {
    pub toolchain_dir: PathBuf,
}

static ENV_INSTALL_CANCEL_REQUESTED: AtomicBool = AtomicBool::new(false);
static ENV_INSTALL_CURRENT_PID: AtomicU32 = AtomicU32::new(0);

fn kill_pid_tree(pid: u32) {
    if pid == 0 {
        return;
    }
    println!("[Epris] Killing EnvInstall process tree for PID: {}", pid);

    #[cfg(target_os = "windows")]
    {
        #[cfg(target_os = "windows")]
        use std::os::windows::process::CommandExt;

        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .status();
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status();
    }
}

fn env_install_canceled() -> bool {
    ENV_INSTALL_CANCEL_REQUESTED.load(Ordering::SeqCst)
}

#[tauri::command]
pub fn cancel_env_install() -> Result<(), String> {
    ENV_INSTALL_CANCEL_REQUESTED.store(true, Ordering::SeqCst);
    let pid = ENV_INSTALL_CURRENT_PID.swap(0, Ordering::SeqCst);
    if pid != 0 {
        kill_pid_tree(pid);
    }
    Ok(())
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

    // New run: clear any previous cancel request.
    ENV_INSTALL_CANCEL_REQUESTED.store(false, Ordering::SeqCst);
    ENV_INSTALL_CURRENT_PID.store(0, Ordering::SeqCst);

    let app_dir = window
        .app_handle()
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?;
    let ws_path = std::path::PathBuf::from(&workspace_path);
    install_log::begin_env_install(
        Some(&window),
        Some(&app_dir),
        Some(&ws_path),
        &provider,
        &workspace_path,
    );
    
    // Simulate stages as per spec
    let stages = [
        ("Detecting environment", 10),
        ("Installing toolchain", 30),
        ("Bootstrapping workspace", 60),
        ("Installing baseline packages", 78),
        ("Installing Remotion skills", 90),
        ("Finalizing", 100),
    ];

    for (step, percent) in stages {
        if env_install_canceled() {
            install_log::log_env_install(
                Some(&window),
                Some(&app_dir),
                Some(&ws_path),
                "Environment install canceled.",
            );
            return Err("Environment install canceled.".to_string());
        }
        let _ = window.emit("env_install_progress", serde_json::json!({
            "step": step,
            "percent": percent
        }));
        install_log::log_env_install(
            Some(&window),
            Some(&app_dir),
            Some(&ws_path),
            &format!("Stage: {}", step),
        );

        if step == "Bootstrapping workspace" {
            // If node_modules already exists, we can skip pnpm install here.
            // This keeps “Install missing dep” fast when only skills are missing.
            let ws_path = std::path::PathBuf::from(&workspace_path);
            let deps_ok = ws_path.join("node_modules").join(".pnpm").exists()
                || ws_path.join("node_modules").join(".bin").exists()
                || ws_path.join("node_modules").exists();
            if deps_ok {
                install_log::log_env_install(
                    Some(&window),
                    Some(&app_dir),
                    Some(&ws_path),
                    "Bootstrapping workspace: skipped (node_modules already present).",
                );
            } else {
                let env_manager = EnvironmentManager::new(app_dir.clone());
                run_pnpm_install(&window, &workspace_path, &env_manager).await?;
            }
        }

        if step == "Installing baseline packages" {
            let env_manager = EnvironmentManager::new(app_dir.clone());
            ensure_baseline_packages(&window, &workspace_path, &env_manager).await?;

            // Also seed the app-local workspace template (so future new projects start with
            // baseline deps already declared in package.json/pnpm-lock.yaml). Best-effort.
            if let Some(template_dir) = projects::get_mutable_template_dir(&window.app_handle())? {
                let template_path = template_dir.to_string_lossy().to_string();
                ensure_baseline_packages(&window, &template_path, &env_manager).await?;
                cleanup_template_runtime_artifacts(&template_dir);
            }
        }

        if step == "Installing toolchain" {
            toolchain::ensure_local_toolchain(
                Some(&window),
                Some(&ws_path),
                &provider,
                &window.app_handle(),
            )
            .await?;
        }

        if step == "Installing Remotion skills" {
            let env_manager = EnvironmentManager::new(app_dir.clone());
            crate::skills::SkillsManager::install_remotion_skills(
                Some(&window),
                &workspace_path,
                &provider,
                &env_manager,
            )
            .await?;
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    }

    Ok(())
}

#[tauri::command]
pub async fn install_js_packages(
    window: tauri::Window,
    workspace_path: String,
    packages: Vec<String>,
) -> Result<(), String> {
    let app_dir = window
        .app_handle()
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?;
    let env_manager = EnvironmentManager::new(app_dir);

    let pkgs: Vec<String> = packages
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if pkgs.is_empty() {
        return Ok(());
    }

    let _ = window.emit(
        "env_install_progress",
        serde_json::json!({ "step": "Installing dependencies", "percent": 0 }),
    );
    let _ = window.emit(
        "env_install_progress",
        serde_json::json!({ "step": format!("pnpm add {}", pkgs.join(", ")), "percent": 10 }),
    );

    let app_data_dir = install_log::toolchain_dir_to_app_data_dir(&env_manager.toolchain_dir);
    let ws_path = std::path::Path::new(&workspace_path);
    install_log::log_env_install(
        Some(&window),
        app_data_dir.as_deref(),
        Some(ws_path),
        &format!("Installing JS packages (user-approved): {}", pkgs.join(", ")),
    );

    match run_pnpm_add(&window, &workspace_path, &env_manager, &pkgs).await {
        Ok(()) => {
            let _ = window.emit(
                "env_install_progress",
                serde_json::json!({ "step": "Dependencies installed", "percent": 100 }),
            );
            Ok(())
        }
        Err(e) => {
            let _ = window.emit(
                "env_install_progress",
                serde_json::json!({ "step": "Dependency install failed", "percent": 100 }),
            );
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn link_workspace_dependencies(
    window: tauri::Window,
    workspace_path: String,
) -> Result<(), String> {
    let app_dir = window
        .app_handle()
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?;
    let env_manager = EnvironmentManager::new(app_dir);

    let _ = window.emit(
        "env_install_progress",
        serde_json::json!({ "step": "Linking dependencies", "percent": 10 }),
    );
    run_pnpm_install(&window, &workspace_path, &env_manager).await?;
    let _ = window.emit(
        "env_install_progress",
        serde_json::json!({ "step": "Dependencies ready", "percent": 100 }),
    );
    Ok(())
}

fn cleanup_template_runtime_artifacts(template_dir: &std::path::Path) {
    // Keep the template clean so newly created projects don’t inherit logs or node_modules.
    let _ = std::fs::remove_dir_all(template_dir.join("logs"));
    let _ = std::fs::remove_dir_all(template_dir.join("node_modules"));
}

fn workspace_store_dir(ws_path: &std::path::Path) -> std::path::PathBuf {
    ws_path
        .parent()
        .map(|p| p.join(".pnpm-store"))
        .unwrap_or_else(|| ws_path.join(".pnpm-store"))
}

fn baseline_signature(app_version: &str) -> String {
    let joined = BASELINE_NPM_PACKAGES
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join("|");
    format!("baseline-v1:{}@app:{}", joined, app_version)
}

fn missing_baseline_packages_for_dir(ws_path: &std::path::Path) -> Result<Vec<String>, String> {
    let pkg_json_path = ws_path.join("package.json");
    if !pkg_json_path.exists() {
        return Ok(Vec::new());
    }

    let bytes = std::fs::read(&pkg_json_path).map_err(|e| e.to_string())?;
    let pkg: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;

    let mut installed = std::collections::HashSet::new();
    for key in ["dependencies", "devDependencies"] {
        if let Some(obj) = pkg.get(key).and_then(|v| v.as_object()) {
            for (k, _) in obj.iter() {
                installed.insert(k.to_string());
            }
        }
    }

    Ok(BASELINE_NPM_PACKAGES
        .iter()
        .filter(|p| !installed.contains(&p.to_string()))
        .map(|p| p.to_string())
        .collect())
}

#[tauri::command]
pub fn get_baseline_packages_info(
    app_handle: tauri::AppHandle,
    workspace_path: String,
) -> Result<BaselinePackagesInfo, String> {
    let signature = baseline_signature(&app_handle.package_info().version.to_string());
    let ws_path = std::path::PathBuf::from(&workspace_path);
    let missing_in_workspace = missing_baseline_packages_for_dir(&ws_path)?;

    let mut missing_in_template: Vec<String> = Vec::new();
    if let Some(template_dir) = projects::get_mutable_template_dir(&app_handle)? {
        missing_in_template = missing_baseline_packages_for_dir(&template_dir)?;
    }

    Ok(BaselinePackagesInfo {
        signature,
        missing_in_workspace,
        missing_in_template,
    })
}

#[tauri::command]
pub async fn install_baseline_packages(
    window: tauri::Window,
    workspace_path: String,
) -> Result<BaselinePackagesInstallResult, String> {
    let signature = baseline_signature(&window.app_handle().package_info().version.to_string());
    let app_dir = window
        .app_handle()
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?;
    let env_manager = EnvironmentManager::new(app_dir);

    let mut installed_in_workspace: Vec<String> = Vec::new();
    let mut installed_in_template: Vec<String> = Vec::new();

    let ws_path = std::path::PathBuf::from(&workspace_path);
    let missing_ws = missing_baseline_packages_for_dir(&ws_path)?;
    if !missing_ws.is_empty() {
        let _ = window.emit(
            "env_install_progress",
            serde_json::json!({ "step": "Installing baseline packages (workspace)", "percent": 30 }),
        );
        run_pnpm_add(&window, &workspace_path, &env_manager, &missing_ws).await?;
        installed_in_workspace = missing_ws;
    }

    if let Some(template_dir) = projects::get_mutable_template_dir(&window.app_handle())? {
        let missing_template = missing_baseline_packages_for_dir(&template_dir)?;
        if !missing_template.is_empty() {
            let template_path = template_dir.to_string_lossy().to_string();
            let _ = window.emit(
                "env_install_progress",
                serde_json::json!({ "step": "Installing baseline packages (template)", "percent": 70 }),
            );
            run_pnpm_add(&window, &template_path, &env_manager, &missing_template).await?;
            cleanup_template_runtime_artifacts(&template_dir);
            installed_in_template = missing_template;
        }
    }

    let _ = window.emit(
        "env_install_progress",
        serde_json::json!({ "step": "Baseline packages ready", "percent": 100 }),
    );

    Ok(BaselinePackagesInstallResult {
        signature,
        installed_in_workspace,
        installed_in_template,
    })
}

async fn ensure_baseline_packages(
    window: &tauri::Window,
    workspace_path: &str,
    env_manager: &EnvironmentManager,
) -> Result<(), String> {
    let app_data_dir = install_log::toolchain_dir_to_app_data_dir(&env_manager.toolchain_dir);
    let ws_path = std::path::Path::new(workspace_path).to_path_buf();

    let pkg_json_path = ws_path.join("package.json");
    if !pkg_json_path.exists() {
        install_log::log_env_install(
            Some(window),
            app_data_dir.as_deref(),
            Some(&ws_path),
            "Baseline packages: package.json not found; skipping.",
        );
        return Ok(());
    }

    let missing = missing_baseline_packages_for_dir(&ws_path)?;

    if missing.is_empty() {
        install_log::log_env_install(
            Some(window),
            app_data_dir.as_deref(),
            Some(&ws_path),
            "Baseline packages: already installed.",
        );
        return Ok(());
    }

    install_log::log_env_install(
        Some(window),
        app_data_dir.as_deref(),
        Some(&ws_path),
        &format!("Baseline packages: installing {}...", missing.join(", ")),
    );

    run_pnpm_add(window, workspace_path, env_manager, &missing).await?;
    Ok(())
}

async fn run_pnpm_add(
    window: &tauri::Window,
    workspace_path: &str,
    env_manager: &EnvironmentManager,
    packages: &[String],
) -> Result<(), String> {
    let app_data_dir = install_log::toolchain_dir_to_app_data_dir(&env_manager.toolchain_dir);
    let ws_path = std::path::Path::new(workspace_path).to_path_buf();

    let store_dir = workspace_store_dir(&ws_path);
    let _ = std::fs::create_dir_all(&store_dir);
    install_log::log_env_install(
        Some(window),
        app_data_dir.as_deref(),
        Some(&ws_path),
        &format!("pnpm store: {}", store_dir.to_string_lossy()),
    );

    let logs_dir = ws_path.join("logs");
    let _ = std::fs::create_dir_all(&logs_dir);
    let pnpm_log_path = logs_dir.join("pnpm-add.log");
    let pnpm_log = std::sync::Arc::new(std::sync::Mutex::new(
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&pnpm_log_path)
            .map_err(|e| e.to_string())?,
    ));

    let bin_dir = env_manager.get_bin_dir();
    let path_env = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{};{}", bin_dir.to_string_lossy(), path_env);

    let store_dir_arg = store_dir.to_string_lossy().to_string();
    let mut args: Vec<String> = Vec::new();
    args.push("add".into());
    args.push("--save-exact".into());
    args.push("--prefer-offline".into());
    args.push("--store-dir".into());
    args.push(store_dir_arg);
    for p in packages {
        args.push(p.clone());
    }

    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let mut child = crate::utils::create_async_shell_command("pnpm", &args_ref)
        .current_dir(workspace_path)
        .env("PATH", &new_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn pnpm add: {}", e))?;

    ENV_INSTALL_CURRENT_PID.store(child.id().unwrap_or(0), Ordering::SeqCst);

    let stdout = child.stdout.take().ok_or("Failed to capture pnpm stdout (add)")?;
    let stderr = child.stderr.take().ok_or("Failed to capture pnpm stderr (add)")?;
    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    let w1 = window.clone();
    let log1 = pnpm_log.clone();
    let app_dir1 = app_data_dir.clone();
    let ws1 = ws_path.clone();
    let stdout_task = tokio::spawn(async move {
        while let Ok(Some(line)) = stdout_reader.next_line().await {
            if let Ok(mut f) = log1.lock() {
                let _ = writeln!(f, "[stdout] {}", line);
            }
            install_log::log_env_install(Some(&w1), app_dir1.as_deref(), Some(&ws1), &line);
        }
    });

    let w2 = window.clone();
    let log2 = pnpm_log.clone();
    let app_dir2 = app_data_dir.clone();
    let ws2 = ws_path.clone();
    let stderr_task = tokio::spawn(async move {
        while let Ok(Some(line)) = stderr_reader.next_line().await {
            if let Ok(mut f) = log2.lock() {
                let _ = writeln!(f, "[stderr] {}", line);
            }
            install_log::log_env_install(Some(&w2), app_dir2.as_deref(), Some(&ws2), &line);
        }
    });

    let cancel_watch = tokio::spawn(async move {
        let mut tick = tokio::time::interval(tokio::time::Duration::from_millis(250));
        loop {
            tick.tick().await;
            if env_install_canceled() {
                let pid = ENV_INSTALL_CURRENT_PID.load(Ordering::SeqCst);
                if pid != 0 {
                    kill_pid_tree(pid);
                }
                break;
            }
        }
    });

    let status = child.wait().await.map_err(|e| e.to_string())?;
    ENV_INSTALL_CURRENT_PID.store(0, Ordering::SeqCst);
    let _ = stdout_task.await;
    let _ = stderr_task.await;
    let _ = cancel_watch.await;

    if env_install_canceled() {
        return Err("Environment install canceled.".to_string());
    }

    if !status.success() {
        return Err(format!(
            "pnpm add failed (exit code: {:?}). See {} for details.",
            status.code(),
            pnpm_log_path.to_string_lossy()
        ));
    }

    install_log::log_env_install(
        Some(window),
        app_data_dir.as_deref(),
        Some(&ws_path),
        &format!("pnpm add completed. Log: {}", pnpm_log_path.to_string_lossy()),
    );
    Ok(())
}

async fn run_pnpm_install(
    window: &tauri::Window,
    workspace_path: &str,
    env_manager: &EnvironmentManager,
) -> Result<(), String> {
    let app_data_dir = install_log::toolchain_dir_to_app_data_dir(&env_manager.toolchain_dir);
    let ws_path = std::path::Path::new(workspace_path).to_path_buf();
    install_log::log_env_install(
        Some(window),
        app_data_dir.as_deref(),
        Some(&ws_path),
        "Running pnpm install...",
    );

    // Keep pnpm store on the same drive as the workspace to avoid Windows hardlink issues
    // (e.g. when the default store is on C: but workspace is on D:).
    let store_dir = workspace_store_dir(&ws_path);
    let _ = std::fs::create_dir_all(&store_dir);
    install_log::log_env_install(
        Some(window),
        app_data_dir.as_deref(),
        Some(&ws_path),
        &format!("pnpm store: {}", store_dir.to_string_lossy()),
    );

    let logs_dir = std::path::Path::new(workspace_path).join("logs");
    let _ = std::fs::create_dir_all(&logs_dir);
    let pnpm_log_path = logs_dir.join("pnpm-install.log");
    let pnpm_log = std::sync::Arc::new(std::sync::Mutex::new(
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&pnpm_log_path)
            .map_err(|e| e.to_string())?,
    ));

    let bin_dir = env_manager.get_bin_dir();
    let path_env = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{};{}", bin_dir.to_string_lossy(), path_env);

    let store_dir_arg = store_dir.to_string_lossy().to_string();
    let mut child = crate::utils::create_async_shell_command(
        "pnpm",
        &[
            "install",
            "--frozen-lockfile",
            "--prefer-offline",
            "--reporter",
            "append-only",
            "--store-dir",
            &store_dir_arg,
        ],
    )
        .current_dir(workspace_path)
        .env("PATH", &new_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn pnpm install: {}", e))?;

    ENV_INSTALL_CURRENT_PID.store(child.id().unwrap_or(0), Ordering::SeqCst);

    // Heartbeat: pnpm can appear "stuck" while doing filesystem work (especially on Windows).
    // Emit periodic progress + log lines so users know the installer is still running.
    let started_at = std::time::Instant::now();
    let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let done_hb = done.clone();
    let w_hb = window.clone();
    let log_hb = pnpm_log.clone();
    let app_dir_hb = app_data_dir.clone();
    let ws_hb = ws_path.clone();
    let heartbeat_task = tokio::spawn(async move {
        let mut tick = tokio::time::interval(tokio::time::Duration::from_secs(8));
        loop {
            tick.tick().await;
            if done_hb.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
            let secs = started_at.elapsed().as_secs();
            let line = format!("pnpm install still running ({}s)...", secs);
            if let Ok(mut f) = log_hb.lock() {
                let _ = writeln!(f, "[heartbeat] {}", line);
            }
            install_log::log_env_install(Some(&w_hb), app_dir_hb.as_deref(), Some(&ws_hb), &line);
            let _ = w_hb.emit(
                "env_install_progress",
                serde_json::json!({ "step": line, "percent": 60 }),
            );
        }
    });

    let stdout = child.stdout.take().ok_or("Failed to capture pnpm stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture pnpm stderr")?;
    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    let w1 = window.clone();
    let log1 = pnpm_log.clone();
    let app_dir1 = app_data_dir.clone();
    let ws1 = ws_path.clone();
    let stdout_task = tokio::spawn(async move {
        while let Ok(Some(line)) = stdout_reader.next_line().await {
            if let Ok(mut f) = log1.lock() {
                let _ = writeln!(f, "[stdout] {}", line);
            }
            install_log::log_env_install(Some(&w1), app_dir1.as_deref(), Some(&ws1), &line);
        }
    });

    let w2 = window.clone();
    let log2 = pnpm_log.clone();
    let app_dir2 = app_data_dir.clone();
    let ws2 = ws_path.clone();
    let stderr_task = tokio::spawn(async move {
        while let Ok(Some(line)) = stderr_reader.next_line().await {
            if let Ok(mut f) = log2.lock() {
                let _ = writeln!(f, "[stderr] {}", line);
            }
            install_log::log_env_install(Some(&w2), app_dir2.as_deref(), Some(&ws2), &line);
        }
    });

    let cancel_watch = tokio::spawn(async move {
        let mut tick = tokio::time::interval(tokio::time::Duration::from_millis(250));
        loop {
            tick.tick().await;
            if env_install_canceled() {
                let pid = ENV_INSTALL_CURRENT_PID.load(Ordering::SeqCst);
                if pid != 0 {
                    kill_pid_tree(pid);
                }
                break;
            }
        }
    });

    let status = child.wait().await.map_err(|e| e.to_string())?;
    ENV_INSTALL_CURRENT_PID.store(0, Ordering::SeqCst);
    done.store(true, std::sync::atomic::Ordering::SeqCst);
    let _ = stdout_task.await;
    let _ = stderr_task.await;
    let _ = heartbeat_task.await;
    let _ = cancel_watch.await;

    if env_install_canceled() {
        return Err("Environment install canceled.".to_string());
    }

    if !status.success() {
        // Best-effort: on Windows, we sometimes see `UNKNOWN: unknown error, open ...` with a
        // negative exit code, leaving a partially written node_modules. Repair once by removing
        // node_modules and retrying.
        let should_retry = status.code() == Some(-4094);
        if should_retry {
            install_log::log_env_install(
                Some(window),
                app_data_dir.as_deref(),
                Some(&ws_path),
                "pnpm install failed; attempting repair: removing workspace node_modules and retrying once...",
            );
            let _ = std::fs::remove_dir_all(ws_path.join("node_modules"));

            let mut retry_child = crate::utils::create_async_shell_command(
                "pnpm",
                &[
                    "install",
                    "--frozen-lockfile",
                    "--prefer-offline",
                    "--store-dir",
                    &store_dir_arg,
                    "--force",
                ],
            )
            .current_dir(workspace_path)
            .env("PATH", &new_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn pnpm install (retry): {}", e))?;

            let stdout = retry_child
                .stdout
                .take()
                .ok_or("Failed to capture pnpm stdout (retry)")?;
            let stderr = retry_child
                .stderr
                .take()
                .ok_or("Failed to capture pnpm stderr (retry)")?;
            let mut stdout_reader = BufReader::new(stdout).lines();
            let mut stderr_reader = BufReader::new(stderr).lines();

            let w1 = window.clone();
            let log1 = pnpm_log.clone();
            let app_dir1 = app_data_dir.clone();
            let ws1 = ws_path.clone();
            let stdout_task = tokio::spawn(async move {
                while let Ok(Some(line)) = stdout_reader.next_line().await {
                    if let Ok(mut f) = log1.lock() {
                        let _ = writeln!(f, "[retry][stdout] {}", line);
                    }
                    install_log::log_env_install(Some(&w1), app_dir1.as_deref(), Some(&ws1), &line);
                }
            });

            let w2 = window.clone();
            let log2 = pnpm_log.clone();
            let app_dir2 = app_data_dir.clone();
            let ws2 = ws_path.clone();
            let stderr_task = tokio::spawn(async move {
                while let Ok(Some(line)) = stderr_reader.next_line().await {
                    if let Ok(mut f) = log2.lock() {
                        let _ = writeln!(f, "[retry][stderr] {}", line);
                    }
                    install_log::log_env_install(Some(&w2), app_dir2.as_deref(), Some(&ws2), &line);
                }
            });

            let retry_status = retry_child.wait().await.map_err(|e| e.to_string())?;
            let _ = stdout_task.await;
            let _ = stderr_task.await;

            if retry_status.success() {
                install_log::log_env_install(
                    Some(window),
                    app_data_dir.as_deref(),
                    Some(&ws_path),
                    "pnpm install completed after repair retry.",
                );
                return Ok(());
            }
        }

        install_log::log_env_install(
            Some(window),
            app_data_dir.as_deref(),
            Some(&ws_path),
            &format!(
                "pnpm install failed (exit code: {:?}). See {} for details.",
                status.code(),
                pnpm_log_path.to_string_lossy()
            ),
        );
        return Err(format!(
            "pnpm install failed (exit code: {:?}). See {} for details.",
            status.code(),
            pnpm_log_path.to_string_lossy()
        ));
    }

    install_log::log_env_install(
        Some(window),
        app_data_dir.as_deref(),
        Some(&ws_path),
        "pnpm install completed.",
    );
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
