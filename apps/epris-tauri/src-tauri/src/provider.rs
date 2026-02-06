use serde::{Serialize, Deserialize};
use std::process::{Child, Command};
use std::path::{PathBuf};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use tauri::Manager;
use crate::environment::EnvironmentManager;
use crate::utils;

static PROVIDER_CLI_CANCEL_REQUESTED: AtomicBool = AtomicBool::new(false);
static PROVIDER_CLI_PID: AtomicU32 = AtomicU32::new(0);

fn kill_pid_tree(pid: u32) {
    if pid == 0 {
        return;
    }
    println!("[Epris] Killing provider CLI process tree for PID: {}", pid);

    #[cfg(target_os = "windows")]
    {
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

pub fn cancel_provider_cli() {
    PROVIDER_CLI_CANCEL_REQUESTED.store(true, Ordering::SeqCst);
    let pid = PROVIDER_CLI_PID.swap(0, Ordering::SeqCst);
    if pid != 0 {
        kill_pid_tree(pid);
    }
}

#[tauri::command]
pub fn stop_provider_cli() -> Result<(), String> {
    cancel_provider_cli();
    Ok(())
}

pub struct ProviderState {
    pub process: Option<Child>,
    pub port: u16,
    pub session_id: Option<String>,
    pub name: String,
    pub model: Option<String>,
    pub is_server: bool,
    pub is_busy: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ProviderType {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub name: String,
    pub is_running: bool,
}

impl Default for ProviderState {
    fn default() -> Self {
        Self { 
            process: None, 
            port: 4096, 
            session_id: None, 
            name: "opencode".into(),
            model: None,
            is_server: true,
            is_busy: false,
        }
    }
}

pub struct ProviderManager;

impl ProviderManager {
    pub fn start(
        state: &mut ProviderState,
        workspace_path: &str,
        provider_name: &str,
        env_manager: &EnvironmentManager,
    ) -> Result<u16, String> {
        // Ensure Rule Files exist
        Self::ensure_rule_files(workspace_path)?;

        if state.process.is_some() || (!state.is_server && state.name == provider_name) {
            if state.name == provider_name {
                return Ok(state.port);
            }
            Self::stop(state)?;
        }

        let port = utils::find_available_port(4096);
        let bin_dir = env_manager.get_bin_dir();
        let path_env = std::env::var("PATH").unwrap_or_default();
        let new_path = format!("{};{}", bin_dir.to_string_lossy(), path_env);

        let prog = if provider_name == "gemini" { "gemini" } else { "opencode" };
        let is_server = provider_name == "opencode";
        state.is_server = is_server;

        if !is_server {
            state.name = provider_name.to_string();
            state.port = 0;
            return Ok(0);
        }

        let mut child_cmd = {
            #[cfg(target_os = "windows")]
            {
                let mut c = Command::new("cmd");
                c.arg("/C").arg(prog);
                c.creation_flags(0x08000000);
                c
            }
            #[cfg(not(target_os = "windows"))]
            {
                Command::new(prog)
            }
        };

        child_cmd.env("PATH", new_path);
        child_cmd.current_dir(workspace_path);
        
        child_cmd.arg("serve")
                 .arg("--hostname").arg("127.0.0.1")
                 .arg("--port").arg(&port.to_string())
                 .arg("--print-logs")
                 .arg("--log-level").arg("DEBUG");

        // Redirect logs
        let logs_dir = std::path::Path::new(workspace_path).join("logs");
        let _ = std::fs::create_dir_all(&logs_dir);
        let log_file_path = logs_dir.join(format!("{}.log", provider_name));
        
        if let Ok(file) = std::fs::OpenOptions::new().create(true).append(true).open(&log_file_path) {
            child_cmd.stdout(file.try_clone().unwrap());
            child_cmd.stderr(file);
        }

        let child = child_cmd.spawn().map_err(|e| format!("Failed to start provider {}: {}", provider_name, e))?;
        
        state.process = Some(child);
        state.port = port;
        state.name = provider_name.to_string();
        
        Ok(port)
    }

    pub fn ensure_rule_files(workspace_path: &str) -> Result<(), String> {
        let ws = std::path::Path::new(workspace_path);
        let remotion_md = ws.join("REMOTION.md");
        let gemini_dir = ws.join(".gemini");
        let opencode_dir = ws.join(".opencode");
        let gemini_settings = gemini_dir.join("settings.json");
        let opencode_config = opencode_dir.join("opencode.json");
        let non_interactive_note = "\n\
                **Important:** The user cannot respond to questions or confirm plans in the middle of this run.\n\
                Do **not** ask for confirmation or wait for replies; execute your plan end-to-end using tools (read/edit files, then verify).\n\n";

        // Clean up legacy GEMINI.md if it exists
        let old_gemini_md = ws.join("GEMINI.md");
        if old_gemini_md.exists() {
            let _ = std::fs::remove_file(old_gemini_md);
        }

        if !remotion_md.exists() {
            let rules = "# Remotion Minimal Rules\n\n\
                ## Goal\n\n\
                Enable fast, safe Remotion video generation for an initial version. Prioritize simplicity and successful rendering over architectural completeness.\n\n\
                **Important:** The user cannot respond to questions or confirm plans in the middle of this run.\n\
                Do **not** ask for confirmation or wait for replies; execute your plan end-to-end using tools (read/edit files, then verify).\n\n\
                ## Dependency Policy (Important)\n\n\
                - DO NOT install npm packages automatically.\n\
                - DO NOT run pnpm/npm/yarn/bun.\n\
                - DO NOT modify package.json or pnpm-lock.yaml.\n\
                - If you need a dependency, ask the user to install it (e.g. \"Please install: <package>\").\n\
                - If the user explicitly requests a specific npm package, you MUST use it via import.\n\
                  Do NOT re-implement it or switch libraries to avoid the dependency; instead request it.\n\n\
                ## 1. Scope\n\n\
                - Only modify files inside `src/**` and `public/**`.\n\
                - All images, audio, and fonts must live in `public/` and be referenced with `staticFile()`.\n\
                - Do not create standalone HTML files or scripts outside `src`.\n\n\
                ## 1.1 UI Props (Epris)\n\n\
                - Epris can expose tunable props via two files:\n\
                  - `src/epris-controls.json` (control definitions)\n\
                  - `src/epris-props.json` (saved values, used for export)\n\
                - If you add or change exposed props, update both files and ensure `src/Root.tsx` passes `defaultProps`.\n\
                - Optional: wrap major objects with `<EprisGroup id label kind>...</EprisGroup>` so the app can list them.\n\n\
                ## 2. Entry Files\n\n\
                ### `src/Root.tsx`\n\n\
                **Allowed:**\n\
                - Adjust `durationInFrames`.\n\
                - Pass data into `Main` via `defaultProps`.\n\n\
                **Rules:**\n\
                - Keep at least one composition that renders `Main`.\n\
                - Do not place animation logic here.\n\n\
                ### `src/VideoConfig.ts` (Configuration)\n\n\
                **Rules:**\n\
                - You MAY modify `DURATION_IN_FRAMES` to change video length.\n\
                - You MUST NOT modify `VIDEO_WIDTH`, `VIDEO_HEIGHT`, or `VIDEO_FPS`.\n\
                - DO NOT modify `Root.tsx` or `Preview.tsx` for configuration changes.\n\n\
                ### `src/Composition.tsx`\n\n\
                **Rules:**\n\
                - `Main` is the primary place to implement animations.\n\
                - For simple videos or single segments, all logic may live directly inside `Main`.\n\
                - Use Remotion APIs such as `useCurrentFrame`, `interpolate`, and `spring`.\n\n\
                ## 3. Complexity Threshold\n\n\
                - If an animation fits naturally in one component, keep it in `Main`.\n\
                - Only create extra files (components or scenes) when the code becomes hard to read or reuse.\n\
                - Avoid premature abstraction.\n\n\
                ## 4. Integrity\n\n\
                - The project must compile and preview after every change.\n\
                - Remove unused imports and avoid obvious TypeScript errors.\n\
                - Ensure timeline length matches `durationInFrames`.\n\n\
                ## 5. Principle\n\n\
                - Favor working output and clarity over perfect structure. Start simple; introduce structure only when necessary.";
            std::fs::write(&remotion_md, rules).map_err(|e| e.to_string())?;
        } else if let Ok(existing) = std::fs::read_to_string(&remotion_md) {
            // Keep this idempotent: only inject the note if it's missing.
            // This helps reduce "plan-only then exit" behavior for single-run CLIs.
            let needs_note = !existing.contains("The user cannot respond to questions or confirm plans")
                && !existing.contains("Do **not** ask for confirmation or wait for replies");
            if needs_note {
                let updated = if let Some(idx) = existing.find("## Dependency Policy") {
                    let mut s = existing.clone();
                    s.insert_str(idx, non_interactive_note);
                    s
                } else {
                    format!("{}{}", non_interactive_note.trim_start(), existing)
                };
                let _ = std::fs::write(&remotion_md, updated);
            }
        }

        if !gemini_dir.exists() {
            std::fs::create_dir_all(&gemini_dir).map_err(|e| e.to_string())?;
        }

        if !opencode_dir.exists() {
            std::fs::create_dir_all(&opencode_dir).map_err(|e| e.to_string())?;
        }

        // Gemini Official Config
        if !gemini_settings.exists() {
            let settings = serde_json::json!({
                "model": {
                    "name": "gemini-3-flash-preview"
                },
                "context": {
                    "fileName": ["REMOTION.md", "package.json"]
                },
                "experimental": {
                    "skills": true
                },
                "tools": {
                    "approvalMode": "auto_edit",
                    "autoAccept": false
                }
            });
            let content = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
            std::fs::write(&gemini_settings, content).map_err(|e| e.to_string())?;
        }

        // OpenCode Official Config
        if !opencode_config.exists() {
            let config = serde_json::json!({
                "instructions": ["REMOTION.md", "package.json"]
            });
            let content = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
            std::fs::write(&opencode_config, content).map_err(|e| e.to_string())?;
        }

        Ok(())
    }
    
    pub fn stop(state: &mut ProviderState) -> Result<(), String> {
        if let Some(mut child) = state.process.take() {
            utils::kill_process_tree(&mut child);
        }
        state.session_id = None;
        Ok(())
    }
}

pub async fn run_provider_cli(
    workspace_path: &str,
    provider_name: &str,
    prompt: &str,
    model_id: Option<String>,
    app_data_dir: PathBuf,
) -> Result<(), String> {
    PROVIDER_CLI_CANCEL_REQUESTED.store(false, Ordering::SeqCst);
    PROVIDER_CLI_PID.store(0, Ordering::SeqCst);

    let env_manager = EnvironmentManager::new(app_data_dir.clone());
    let bin_dir = env_manager.get_bin_dir();
    let path_env = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{};{}", bin_dir.to_string_lossy(), path_env);

    // Context is now handled via settings.json for both Gemini and OpenCode
    // We pass the raw prompt directly to avoid "Acknowledged" responses
    let final_prompt = prompt.to_string();

    let mut args = Vec::new();
    args.push(final_prompt);
    args.push("--approval-mode".to_string());
    args.push("yolo".to_string());
    
    if let Some(mid) = model_id {
        args.push("--model".to_string());
        args.push(mid);
    }
    
    args.push("--output-format".to_string());
    args.push("json".to_string());
    args.push("--debug".to_string());

    let prog = if provider_name == "gemini" { "gemini" } else { "opencode" };
    let mut cmd = {
        #[cfg(target_os = "windows")]
        {
            let mut c = Command::new("cmd");
            c.arg("/C").arg(prog);
            c.creation_flags(0x08000000);
            c
        }
        #[cfg(not(target_os = "windows"))]
        {
            Command::new(prog)
        }
    };

    cmd.args(args);
    cmd.env("PATH", new_path);
    
    // Use API Key from state if available
    let state_mgr = crate::state_manager::StateManager::new(app_data_dir);
    if let Some(key) = state_mgr.read().gemini_api_key {
        cmd.env("GOOGLE_API_KEY", key);
    }

    cmd.current_dir(workspace_path);

    // Redirect logs for CLI too
    let log_file_path = std::path::Path::new(workspace_path).join("logs").join(format!("{}.log", provider_name));
    if let Ok(file) = std::fs::OpenOptions::new().create(true).append(true).open(&log_file_path) {
        cmd.stdout(file.try_clone().unwrap());
        cmd.stderr(file);
    }

    if PROVIDER_CLI_CANCEL_REQUESTED.load(Ordering::SeqCst) {
        return Err("Canceled".to_string());
    }

    let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn CLI provider: {}", e))?;
    PROVIDER_CLI_PID.store(child.id(), Ordering::SeqCst);

    let status = child.wait().map_err(|e| format!("Failed to wait CLI provider: {}", e))?;
    PROVIDER_CLI_PID.store(0, Ordering::SeqCst);

    if !status.success() {
        if PROVIDER_CLI_CANCEL_REQUESTED.load(Ordering::SeqCst) {
            return Err("Canceled".to_string());
        }
        return Err(format!("CLI provider {} failed with exit code {:?}", provider_name, status.code()));
    }

    Ok(())
}

#[tauri::command]
pub async fn start_provider_cmd(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, Mutex<ProviderState>>,
    workspace_path: String,
    provider: String,
) -> Result<u16, String> {
    let app_dir = app_handle.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let env_manager = EnvironmentManager::new(app_dir);
    
    // Install Skills (Async, No lock)
    if let Err(e) = crate::skills::SkillsManager::install_remotion_skills(None, &workspace_path, &provider, &env_manager).await {
        println!("[Epris] Warning: Failed to install skills: {}", e);
    }

    // Start Provider (Sync, With lock)
    let mut ps = state.lock().map_err(|e| e.to_string())?;
    ProviderManager::start(&mut ps, &workspace_path, &provider, &env_manager)
}
