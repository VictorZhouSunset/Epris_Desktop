use serde::{Serialize, Deserialize};
use std::path::Path;
use std::io::Write;
use std::process::Child;
use std::process::Stdio;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::utils;
use crate::sandbox;
use crate::snapshot;
use crate::gate::{self, GateMode, GateResult};
use tauri::{Manager, Emitter};

static PROMPT_CANCEL_REQUESTED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Serialize)]
struct PromptProgress {
    percent: f64,
    step: String,
}

pub const OPENCODE_SYSTEM_PROMPT: &str = "You are an expert Remotion animation developer. \
    Workspace Structure: \
    - src/Root.tsx: Defines composition entries. DO NOT remove or rename the 'Main' composition record. \
    - src/Composition.tsx: Implementation of animation components. ALWAYS edit the 'Main' component implementation here to change the principal animation. \
    \
    Rules: \
    1. FULFILL user requests by primarily modifying components in src/Composition.tsx. \
    2. Do NOT create new composition records in Root.tsx unless explicitly asked. Focus on the 'Main' composition. \
    3. Ensure the composition still renders successfully after changes (clean imports, valid JSX). \
    4. Use Remotion's animation APIs: useCurrentFrame, interpolate, spring, staticFile, etc. \
    5. Only operate on files inside the 'src' directory. DO NOT create standalone HTML files or files in the root. \
    6. Do NOT install dependencies. Do NOT run pnpm/npm/yarn/bun commands. Do NOT change package.json or pnpm-lock.yaml. \
       If you need a new npm package, write a short request in your response like: \
       DEPENDENCY_REQUEST: <pkg1>, <pkg2> (1-line reason). Then stop. \
    7. If the user explicitly requests using a specific npm package (e.g. \"use simplex-noise\"), you MUST use that package via import. \
       Do NOT re-implement or substitute a different library to avoid the dependency. If it's missing, output DEPENDENCY_REQUEST and stop. \
    \
    The current working directory is the workspace root.";

fn describe_installed_packages(workspace_path: &str) -> String {
    let pkg_path = std::path::Path::new(workspace_path).join("package.json");
    let bytes = match std::fs::read(&pkg_path) {
        Ok(b) => b,
        Err(_) => return "package.json not found".to_string(),
    };
    let v: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => return "package.json unreadable".to_string(),
    };

    let mut deps: Vec<String> = Vec::new();
    let mut dev: Vec<String> = Vec::new();
    if let Some(obj) = v.get("dependencies").and_then(|x| x.as_object()) {
        deps.extend(obj.keys().cloned());
    }
    if let Some(obj) = v.get("devDependencies").and_then(|x| x.as_object()) {
        dev.extend(obj.keys().cloned());
    }
    deps.sort();
    dev.sort();
    format!(
        "dependencies: [{}]\n devDependencies: [{}]\n(If you need a new package, do NOT install; request it.)",
        deps.join(", "),
        dev.join(", ")
    )
}

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

#[derive(Debug, Serialize, Deserialize)]
pub struct PromptResponse {
    pub success: bool,
    pub message: String,
    pub gate_result: Option<GateResult>,
    pub snapshot_id: Option<String>,
    #[serde(default)]
    pub canceled: bool,
    #[serde(default)]
    pub validation_skipped: bool,
}

#[tauri::command]
pub fn cancel_current_run(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, Mutex<OpenCodeState>>,
) -> Result<(), String> {
    PROMPT_CANCEL_REQUESTED.store(true, Ordering::SeqCst);
    let _ = gate::stop_gate_validation();
    let _ = crate::provider::stop_provider_cli();
    let _ = stop_opencode(state);
    let _ = app_handle.emit("prompt-progress", PromptProgress { percent: 0.0, step: "Canceled".into() });
    Ok(())
}

#[tauri::command]
pub fn start_opencode(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, Mutex<OpenCodeState>>,
    workspace_path: String,
) -> Result<u16, String> {
    let mut oc = state.lock().map_err(|e| e.to_string())?;
    
    if oc.process.is_some() {
        return Ok(oc.port);
    }
    
    let port = utils::find_available_port(4096);

    let logs_dir = Path::new(&workspace_path).join("logs");
    let _ = std::fs::create_dir_all(&logs_dir);
    let log_path = logs_dir.join("opencode.log");
    let mut log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| format!("Failed to open opencode.log: {}", e))?;
    let _ = writeln!(log_file, "\n--- OpenCode Server Log [{}] ---", chrono::Utc::now());

    let mut cmd = utils::create_shell_command(
        "opencode",
        &["serve", "--hostname", "127.0.0.1", "--port", &port.to_string()],
    );
    cmd.current_dir(&workspace_path);

    // Ensure the server can find our local toolchain/bin (opencode may not be on system PATH).
    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let toolchain_bin = app_dir.join("toolchain").join("bin");
    let path_sep = if cfg!(target_os = "windows") { ";" } else { ":" };
    let current_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}{}{}", toolchain_bin.to_string_lossy(), path_sep, current_path);
    cmd.env("PATH", new_path);

    // Reduce unreadable ANSI escape sequences in opencode.log.
    cmd.env("NO_COLOR", "1");
    cmd.env("FORCE_COLOR", "0");
    cmd.env("TERM", "dumb");

    cmd.stdout(Stdio::from(log_file.try_clone().map_err(|e| e.to_string())?));
    cmd.stderr(Stdio::from(log_file));

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to start OpenCode: {}", e))?;

    // If the child exits immediately, surface a helpful error.
    if let Ok(Some(status)) = child.try_wait() {
        return Err(format!(
            "OpenCode server exited immediately (code: {:?}). Check workspace/logs/opencode.log",
            status.code()
        ));
    }
    
    oc.process = Some(child);
    oc.port = port;
    
    Ok(port)
}

#[tauri::command]
pub fn stop_opencode(state: tauri::State<'_, Mutex<OpenCodeState>>) -> Result<(), String> {
    let mut oc = state.lock().map_err(|e| e.to_string())?;
    
    if let Some(mut child) = oc.process.take() {
        utils::kill_process_tree(&mut child);
    }
    oc.session_id = None;
    
    Ok(())
}

#[tauri::command]
pub async fn send_prompt(
    state: tauri::State<'_, Mutex<OpenCodeState>>,
    workspace_path: String,
    prompt: String,
    provider: String,
    model: String,
    app_handle: tauri::AppHandle,
) -> Result<PromptResponse, String> {
    PROMPT_CANCEL_REQUESTED.store(false, Ordering::SeqCst);
    let canceled = || PROMPT_CANCEL_REQUESTED.load(Ordering::SeqCst);

    // If provider is Gemini, route exclusively to run_provider_cli
    if provider == "gemini" {
        let app_data_dir = app_handle.path().app_local_data_dir().map_err(|e| e.to_string())?;
             
        // Step 1: Pre-flight backup
        let _ = app_handle.emit("prompt-progress", PromptProgress { percent: 10.0, step: "Creating pre-flight backup".into() });
        println!("[Epris] Step 1/5: Creating pre-flight backup (Gemini)...");
        snapshot::create_backup(&workspace_path, "pre_flight")?;
        let ai_started_at = std::time::SystemTime::now();

        // Step 2: Run CLI
        let _ = app_handle.emit("prompt-progress", PromptProgress { percent: 20.0, step: "AI Generating Animation Code".into() });
        println!("[Epris] Step 2/5: Running Gemini CLI...");
        let start = std::time::Instant::now();
        let cli_res = crate::provider::run_provider_cli(
            &workspace_path, 
            "gemini", 
            &prompt, 
            Some(model.clone()), 
            app_data_dir
        ).await;
        if let Err(e) = cli_res {
            if canceled() || e == "Canceled" {
                return Ok(PromptResponse {
                    success: false,
                    message: "Canceled".into(),
                    gate_result: None,
                    snapshot_id: None,
                    canceled: true,
                    validation_skipped: false,
                });
            }
            return Err(e);
        }
        let duration = start.elapsed().as_secs_f64();
        
        // Log performance
        let _ = log_ai_performance(&workspace_path, &prompt, duration);
        
        // Step 3: Wait
        let _ = app_handle.emit("prompt-progress", PromptProgress { percent: 75.0, step: "Waiting for file system to settle".into() });
        println!("[Epris] Step 3/5: Waiting for file system...");
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        if canceled() {
            return Ok(PromptResponse {
                success: false,
                message: "Canceled".into(),
                gate_result: None,
                snapshot_id: None,
                canceled: true,
                validation_skipped: false,
            });
        }

        // Step 4: Security Check
        let _ = app_handle.emit("prompt-progress", PromptProgress { percent: 85.0, step: "Running security checks".into() });
         println!("[Epris] Step 4/5: Running security check...");
        let backup_path = Path::new(&workspace_path).join(".epris/backups/pre_flight");
        let forbidden_files = utils::check_forbidden_files(&workspace_path, &backup_path)?;

        if !forbidden_files.is_empty() {
             // ... handles same as below ...
             // Refactoring common logic would be good but for now duplication is safer
             println!("[Epris] 🔒 SECURITY VIOLATION: AI modified forbidden files: {:?}", forbidden_files);
             snapshot::restore_backup(&workspace_path, "pre_flight")?;
             return Ok(PromptResponse {
                success: false,
                message: format!("🔒 Security violation: Files protected. Reverted."),
                gate_result: None,
                snapshot_id: None,
                canceled: false,
                validation_skipped: false,
            });
        }

        // Directory & boundary audit (best-effort): forbid node_modules/.git writes, and any writes outside allowed dirs.
        let audit = sandbox::audit_workspace_after_ai_run(Path::new(&workspace_path), ai_started_at)?;
        if audit.incomplete {
            let _ = log_security_audit_note(
                &workspace_path,
                &prompt,
                "Workspace audit was best-effort (timed out scanning large directories).",
            );
        }
        if !audit.violations.is_empty() {
            let _ = log_security_violation(
                &workspace_path,
                &audit.violations,
                &prompt,
                std::collections::HashMap::new(),
            );
            snapshot::restore_backup(&workspace_path, "pre_flight")?;
            return Ok(PromptResponse {
                success: false,
                message: format!(
                    "🔒 Security violation: AI wrote to forbidden paths: {}. Changes reverted.",
                    audit.violations.join(", ")
                ),
                gate_result: None,
                snapshot_id: None,
                canceled: false,
                validation_skipped: false,
            });
        }

        // Step 5: Gate
        let _ = app_handle.emit("prompt-progress", PromptProgress { percent: 95.0, step: "Validating animation integrity".into() });
        println!("[Epris] Step 5/5: Entering Gate Validation loop...");
        let parent_id = snapshot::get_dag_head(workspace_path.clone()).unwrap_or_else(|_| "root".into());
        let draft_snapshot_id = snapshot::auto_save_snapshot(
            workspace_path.clone(),
            prompt.clone(),
            parent_id,
            "gemini".into(),
        )
        .ok();

        let gate_result = gate_loop(workspace_path.clone(), app_handle.clone()).await?;
        if gate_result.error_output == "Gate canceled" && canceled() {
            return Ok(PromptResponse {
                success: false,
                message: "Canceled".into(),
                gate_result: None,
                snapshot_id: draft_snapshot_id,
                canceled: true,
                validation_skipped: false,
            });
        }
        if let Some(id) = draft_snapshot_id.clone() {
            let _ = snapshot::update_dag_snapshot_gate_result(workspace_path.clone(), id, gate_result.clone());
        }

        return Ok(PromptResponse {
            success: gate_result.passed,
            message: "Gemini task completed".into(),
            gate_result: Some(gate_result),
            snapshot_id: draft_snapshot_id,
            canceled: false,
            validation_skipped: false,
        });
    }

    // Existing OpenCode Logic
    let parent_id = snapshot::get_dag_head(workspace_path.clone())?;
    let (port, session_id) = {
        let oc = state.lock().map_err(|e| e.to_string())?;
        (oc.port, oc.session_id.clone())
    };
    
    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{}", port);
    if canceled() {
        return Ok(PromptResponse {
            success: false,
            message: "Canceled".into(),
            gate_result: None,
            snapshot_id: None,
            canceled: true,
            validation_skipped: false,
        });
    }

    // Resolve a valid OpenCode providerID/modelID pair.
    // The UI currently treats "provider" as the app-level provider selector ("opencode" | "gemini")
    // and "model" as an arbitrary string. OpenCode expects a concrete providerID + modelID.
    let (provider_id, model_id) =
        resolve_opencode_model(&client, &base_url, &workspace_path, &provider, &model).await?;
    
    let sid = match session_id {
        Some(id) => id,
        None => {
            let res = match client.post(format!("{}/session", base_url))
                .json(&serde_json::json!({
                    "title": "Epris Remotion"
                }))
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    if canceled() {
                        return Ok(PromptResponse {
                            success: false,
                            message: "Canceled".into(),
                            gate_result: None,
                            snapshot_id: None,
                            canceled: true,
                            validation_skipped: false,
                        });
                    }
                    return Err(format!("Failed to create session: {}", e));
                }
            };
            
            let body: serde_json::Value = res.json().await.map_err(|e| format!("Failed to parse session response: {}", e))?;
            let new_id = body["id"].as_str().ok_or("OpenCode response missing id")?.to_string();
            
            {
                let mut oc = state.lock().map_err(|e| e.to_string())?;
                oc.session_id = Some(new_id.clone());
            }
            println!("[Epris] OpenCode session created: {}", new_id);
            new_id
        }
    };

    println!("[Epris] Step 1/5: Creating pre-flight backup...");
    let _ = app_handle.emit("prompt-progress", PromptProgress { percent: 10.0, step: "Creating pre-flight backup".into() });
    snapshot::create_backup(&workspace_path, "pre_flight")?;
    let ai_started_at = std::time::SystemTime::now();
    
    println!("[Epris] Step 2/5: Sending prompt to AI: \"{}\"", prompt);
    let _ = app_handle.emit("prompt-progress", PromptProgress { percent: 20.0, step: "AI Generating Animation Code".into() });
    
    let remotion_rules = std::fs::read_to_string(std::path::Path::new(&workspace_path).join("REMOTION.md"))
        .unwrap_or_else(|_| OPENCODE_SYSTEM_PROMPT.to_string());

    let prompt_start = std::time::Instant::now();
    let system_prompt = format!(
        "{}\n\nInstalled npm packages:\n{}\n\nThe Remotion workspace is located at: {}. You MUST read src/Composition.tsx and src/Root.tsx.",
        remotion_rules,
        describe_installed_packages(&workspace_path),
        workspace_path
    );

    let res = match client.post(format!("{}/session/{}/message", base_url, sid))
        .json(&serde_json::json!({
            "model": { "providerID": provider_id, "modelID": model_id },
            "system": system_prompt,
            "agent": "build",
            "parts": [{ "type": "text", "text": prompt }]
        }))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            if canceled() {
                return Ok(PromptResponse {
                    success: false,
                    message: "Canceled".into(),
                    gate_result: None,
                    snapshot_id: None,
                    canceled: true,
                    validation_skipped: false,
                });
            }
            return Err(format!("Failed to send prompt: {}", e));
        }
    };
    
    let body: serde_json::Value = res.json().await.map_err(|e| format!("Failed to parse response JSON: {}", e))?;
    let duration = prompt_start.elapsed().as_secs_f64();
    println!("[Epris] Step 2/5: AI response received in {:.2}s", duration);
    
    let _ = log_ai_performance(&workspace_path, &prompt, duration);
    
    let message_text = body["parts"]
        .as_array()
        .and_then(|parts| {
            parts.iter()
                .find(|p| p["type"] == "text")
                .and_then(|p| p["text"].as_str())
        })
        .unwrap_or("Check workspace for changes");
    
    println!("[Epris] Step 3/5: Waiting 3s for file system to settle...");
    let _ = app_handle.emit("prompt-progress", PromptProgress { percent: 75.0, step: "Waiting for file system to settle".into() });
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    println!("[Epris] Step 4/5: Running security check...");
    let _ = app_handle.emit("prompt-progress", PromptProgress { percent: 85.0, step: "Running security checks".into() });
    let backup_path = Path::new(&workspace_path).join(".epris/backups/pre_flight");
    let forbidden_files = utils::check_forbidden_files(&workspace_path, &backup_path)?;
 
    if !forbidden_files.is_empty() {
        println!("[Epris] 🔒 SECURITY VIOLATION: AI modified forbidden files: {:?}", forbidden_files);
        let mut diffs = std::collections::HashMap::new();
        for file in &forbidden_files {
            let ws_path = Path::new(&workspace_path).join(file);
            let bk_path = backup_path.join(file);
            
            if ws_path.exists() && bk_path.exists() {
                let ws_content = std::fs::read_to_string(&ws_path).unwrap_or_default();
                let bk_content = std::fs::read_to_string(&bk_path).unwrap_or_default();
                let diff = utils::generate_line_diff(&bk_content, &ws_content);
                if !diff.is_empty() {
                    diffs.insert(file.clone(), diff);
                }
            }
        }

        log_security_violation(&workspace_path, &forbidden_files, &prompt, diffs)?;
        println!("[Epris] Restoring from pre-flight backup...");
        snapshot::restore_backup(&workspace_path, "pre_flight")?;

        return Ok(PromptResponse {
            success: false,
            message: format!("🔒 Security violation: AI attempted to modify protected files: {}. Changes reverted.", forbidden_files.join(", ")),
            gate_result: None,
            snapshot_id: None,
            canceled: false,
            validation_skipped: false,
        });
    }

    let audit = sandbox::audit_workspace_after_ai_run(Path::new(&workspace_path), ai_started_at)?;
    if audit.incomplete {
        let _ = log_security_audit_note(
            &workspace_path,
            &prompt,
            "Workspace audit was best-effort (timed out scanning large directories).",
        );
    }
    if !audit.violations.is_empty() {
        let _ = log_security_violation(
            &workspace_path,
            &audit.violations,
            &prompt,
            std::collections::HashMap::new(),
        );
        println!("[Epris] Restoring from pre-flight backup...");
        snapshot::restore_backup(&workspace_path, "pre_flight")?;
        return Ok(PromptResponse {
            success: false,
            message: format!(
                "🔒 Security violation: AI wrote to forbidden paths: {}. Changes reverted.",
                audit.violations.join(", ")
            ),
            gate_result: None,
            snapshot_id: None,
            canceled: false,
            validation_skipped: false,
        });
    }

    let draft_snapshot_id = snapshot::auto_save_snapshot(workspace_path.clone(), prompt.clone(), parent_id, sid.clone()).ok();

    println!("[Epris] Step 5/5: Entering Gate Validation loop...");
    let _ = app_handle.emit("prompt-progress", PromptProgress { percent: 95.0, step: "Validating animation integrity".into() });
    let gate_result = gate_loop(workspace_path.clone(), app_handle.clone()).await?;

    if gate_result.error_output == "Gate canceled" {
        if canceled() {
            return Ok(PromptResponse {
                success: false,
                message: "Canceled".into(),
                gate_result: None,
                snapshot_id: draft_snapshot_id,
                canceled: true,
                validation_skipped: false,
            });
        }
        return Ok(PromptResponse {
            success: true,
            message: "Validation skipped".into(),
            gate_result: None,
            snapshot_id: draft_snapshot_id,
            canceled: false,
            validation_skipped: true,
        });
    }

    if let Some(id) = draft_snapshot_id.clone() {
        let _ = snapshot::update_dag_snapshot_gate_result(workspace_path.clone(), id, gate_result.clone());
    }

    Ok(PromptResponse {
        success: gate_result.passed,
        message: message_text.to_string(),
        gate_result: Some(gate_result),
        snapshot_id: draft_snapshot_id,
        canceled: false,
        validation_skipped: false,
    })
}

async fn resolve_opencode_model(
    client: &reqwest::Client,
    base_url: &str,
    workspace_path: &str,
    provider_param: &str,
    model_param: &str,
) -> Result<(String, String), String> {
    let (requested_provider, requested_model) = if let Some((p, m)) = model_param.split_once('/') {
        // Treat model as "providerID/modelID" if possible.
        (p.trim().to_string(), m.trim().to_string())
    } else {
        // Back-compat: treat params as providerID + modelID.
        (provider_param.trim().to_string(), model_param.trim().to_string())
    };

    let providers = match fetch_opencode_providers(client, base_url, workspace_path).await {
        Ok(p) => p,
        Err(e) => {
            println!(
                "[Epris] Warning: failed to query OpenCode providers/models ({}). Sending requested model as-is.",
                e
            );
            return Ok((requested_provider, requested_model));
        }
    };

    if providers.is_empty() {
        println!(
            "[Epris] Warning: OpenCode providers/models list was empty. Sending requested model as-is."
        );
        return Ok((requested_provider, requested_model));
    }

    // First try requested pair (if it exists).
    if let Some(models) = providers.get(&requested_provider) {
        if models.iter().any(|m| m == &requested_model) {
            return Ok((requested_provider, requested_model));
        }
    }

    // Otherwise pick the first available model from any provider.
    for (pid, models) in &providers {
        if let Some(mid) = models.first() {
            println!(
                "[Epris] OpenCode model '{}' not available; falling back to '{}/{}'",
                model_param, pid, mid
            );
            return Ok((pid.clone(), mid.clone()));
        }
    }

    Err("OpenCode has no available models configured. Configure a provider (e.g. OpenAI/Ollama/LM Studio) for opencode, then retry.".to_string())
}

async fn fetch_opencode_providers(
    client: &reqwest::Client,
    base_url: &str,
    workspace_path: &str,
) -> Result<std::collections::BTreeMap<String, Vec<String>>, String> {
    // Try most informative endpoints first. Many support `directory=...`.
    let dir = urlencoding::encode(workspace_path);
    let urls = [
        format!("{}/config/providers?directory={}", base_url, dir),
        format!("{}/config/providers", base_url),
        format!("{}/provider?directory={}", base_url, dir),
        format!("{}/provider", base_url),
    ];

    let mut last_err: Option<String> = None;
    for url in urls {
        match client.get(&url).send().await {
            Ok(res) => {
                if !res.status().is_success() {
                    last_err = Some(format!("{} returned {}", url, res.status()));
                    continue;
                }
                let v: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
                let parsed = parse_providers_models(v);
                if !parsed.is_empty() {
                    return Ok(parsed);
                }
                last_err = Some("Provider list was empty".to_string());
            }
            Err(e) => last_err = Some(e.to_string()),
        }
    }

    Err(format!(
        "Failed to query OpenCode providers/models ({}).",
        last_err.unwrap_or_else(|| "unknown error".to_string())
    ))
}

fn parse_providers_models(v: serde_json::Value) -> std::collections::BTreeMap<String, Vec<String>> {
    use serde_json::Value;
    let mut out: std::collections::BTreeMap<String, Vec<String>> = Default::default();

    // Shape A: {"openai": {"models": {...}}, "ollama": {"models": {...}}}
    if let Some(obj) = v.as_object() {
        if obj.values().all(|vv| vv.is_object()) {
            for (pid, pv) in obj {
                let pid = pid.trim().to_string();
                if pid.is_empty() {
                    continue;
                }
                if let Some(models_obj) = pv.get("models").and_then(|m| m.as_object()) {
                    let mut models: Vec<String> = models_obj.keys().cloned().collect();
                    models.sort();
                    models.dedup();
                    if !models.is_empty() {
                        out.insert(pid, models);
                    }
                }
            }
            if !out.is_empty() {
                return out;
            }
        }
    }

    let providers: Vec<Value> = if let Some(arr) = v.as_array() {
        arr.clone()
    } else if let Some(arr) = v.get("providers").and_then(|p| p.as_array()) {
        arr.clone()
    } else {
        Vec::new()
    };

    for p in providers {
        let pid = p
            .get("id")
            .or_else(|| p.get("providerID"))
            .or_else(|| p.get("name"))
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .trim()
            .to_string();

        if pid.is_empty() {
            continue;
        }

        let mut models: Vec<String> = Vec::new();
        if let Some(mobj) = p.get("models").and_then(|m| m.as_object()) {
            models.extend(mobj.keys().cloned());
        } else if let Some(marr) = p.get("models").and_then(|m| m.as_array()) {
            for m in marr {
                if let Some(s) = m.as_str() {
                    models.push(s.to_string());
                } else if let Some(s) = m.get("id").and_then(|x| x.as_str()) {
                    models.push(s.to_string());
                }
            }
        }

        models.sort();
        models.dedup();
        if !models.is_empty() {
            out.insert(pid, models);
        }
    }

    out
}

#[tauri::command]
pub fn clear_session(state: tauri::State<'_, Mutex<OpenCodeState>>) -> Result<(), String> {
    let mut oc = state.lock().map_err(|e| e.to_string())?;
    oc.session_id = None;
    Ok(())
}

#[allow(dead_code)]
async fn request_fix(
    port: u16,
    session_id: String,
    gate_result: &GateResult,
    provider: String,
    model: String,
) -> Result<(), String> {
    let fix_prompt = format!(
        "The previous changes failed validation. Please fix ONLY the errors below. Do not refactor unrelated code.\n\nErrors:\n{}",
        gate_result.error_output
    );
    
    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{}", port);
    
    let res = client.post(format!("{}/session/{}/message", base_url, session_id))
        .json(&serde_json::json!({
            "model": { "providerID": provider, "modelID": model },
            "system": OPENCODE_SYSTEM_PROMPT,
            "agent": "build",
            "parts": [{ "type": "text", "text": fix_prompt }]
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to send fix request: {}", e))?;
    
    let _ = res.json::<serde_json::Value>().await;
    Ok(())
}

async fn gate_loop(
    workspace_path: String,
    app_handle: tauri::AppHandle,
) -> Result<GateResult, String> {
    // Human-in-the-loop: run Gate once and surface errors to the user.
    let mut result = gate::run_gate_with_mode(workspace_path.clone(), GateMode::Balanced, Some(&app_handle)).await?;
    result.attempts = 1;
    Ok(result)
}

pub fn log_security_violation(
    workspace_path: &str,
    forbidden_files: &[String],
    prompt: &str,
    diffs: std::collections::HashMap<String, String>,
) -> Result<(), String> {
    let log_dir = Path::new(workspace_path).join("logs");
    std::fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;

    let log_path = log_dir.join("security.log");
    let mut log_entry = format!(
        "[{}] 🔒 SECURITY VIOLATION DETECTED\nPrompt: {}\nModified Files: {}\n",
        chrono::Utc::now().to_rfc3339(),
        prompt,
        forbidden_files.join(", ")
    );

    if !diffs.is_empty() {
        for (file, diff) in diffs {
            log_entry.push_str(&format!("\nFile: {}\n{}\n", file, diff));
        }
    }

    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .and_then(|mut file| file.write_all(log_entry.as_bytes()))
        .map_err(|e| e.to_string())
}

fn log_security_audit_note(workspace_path: &str, prompt: &str, note: &str) -> Result<(), String> {
    let log_dir = Path::new(workspace_path).join("logs");
    std::fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;

    let log_path = log_dir.join("security.log");
    let log_entry = format!(
        "[{}] 🔎 AUDIT NOTE\nPrompt: {}\n{}\n",
        chrono::Utc::now().to_rfc3339(),
        prompt,
        note
    );

    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .and_then(|mut file| file.write_all(log_entry.as_bytes()))
        .map_err(|e| e.to_string())
}

pub fn log_ai_performance(
    workspace_path: &str,
    prompt: &str,
    duration_sec: f64,
) -> Result<(), String> {
    let log_dir = Path::new(workspace_path).join("logs");
    std::fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;

    let log_path = log_dir.join("performance.log");
    let log_entry = format!(
        "[{}] TYPE: AI_PROMPT | DURATION: {:.2}s | PROMPT: {}\n",
        chrono::Utc::now().to_rfc3339(),
        duration_sec,
        prompt.replace('\n', " ")
    );

    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .and_then(|mut file| file.write_all(log_entry.as_bytes()))
        .map_err(|e| e.to_string())
}
