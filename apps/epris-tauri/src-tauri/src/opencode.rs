use crate::gate::{self, GateMode, GateResult};
use crate::sandbox;
use crate::snapshot;
use crate::utils;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;
use std::process::Child;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{Emitter, Manager};
use walkdir::WalkDir;

static PROMPT_CANCEL_REQUESTED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Serialize)]
struct PromptProgress {
    percent: f64,
    step: String,
}

pub const OPENCODE_SYSTEM_PROMPT: &str = "You are an expert Remotion animation developer operating inside Epris Desktop. \
    Runtime reality (critical): \
    - The end user is in a GUI and cannot see terminal output, filesystem paths, CLI prompts, approval dialogs, or plan checkpoints. \
    - The end user can only see preview, exposed handles/controls, Projects, and Settings. \
    - Never stop mid-run to ask for confirmation or a reply. Execute end-to-end in one run. \
    \
    Project framework: \
    - Stack: React + TypeScript + Remotion. \
    - src/Root.tsx: Defines composition entries. DO NOT remove or rename the 'Main' composition record. \
    - src/Composition.tsx: Main animation implementation. Prefer changing the 'Main' logic here for user requests. \
    - src/VideoConfig.ts: You MAY adjust DURATION_IN_FRAMES when needed. Do not change FPS/width/height unless explicitly requested. \
    - Epris UI contract files (when touched) must stay in sync: src/epris-controls.json and src/epris-props.json. \
    - Epris control widgets are prebuilt by the host app. Do NOT implement custom controls UI; only expose/edit props via the contract JSON files. \
    \
    Rules: \
    1. FULFILL user requests by primarily modifying components in src/Composition.tsx. \
    2. Do NOT create new composition records in Root.tsx unless explicitly asked. Focus on the 'Main' composition. \
    3. Ensure the composition still renders successfully after changes (clean imports, valid JSX). \
    4. Use Remotion APIs such as useCurrentFrame, interpolate, spring, and staticFile. \
    5. Only operate on files inside src/** and public/**. DO NOT create standalone HTML files or files in the root. \
    6. Do NOT install dependencies. Do NOT run pnpm/npm/yarn/bun commands. Do NOT change package.json or pnpm-lock.yaml. \
       If you need a new npm package, write: DEPENDENCY_REQUEST: <pkg1>, <pkg2> (1-line reason), then stop. \
    7. If the user explicitly requests a specific npm package (e.g. \"use simplex-noise\"), you MUST use it via import. \
       Do NOT re-implement or substitute a different library to avoid the dependency. If missing, output DEPENDENCY_REQUEST and stop. \
    8. If you edit src/epris-controls.json, ui must match type exactly: \
       number -> slider|input, color -> color, select -> select, boolean -> toggle, text -> text|textarea. \
    9. Plan tracking is mandatory: \
       - Create or replace `.epris/agent/active-plan.md` before code edits. \
       - Use Markdown checklist items (`- [ ]`) for steps. \
       - Keep `active-plan.md` user-facing and structural only (elements/scenes/behaviors); no file paths, commands, or coding internals. \
       - Put technical implementation details into internal planning files under `.epris/agent/` (for example: `.epris/agent/task_plan.md`, `.epris/agent/progress.md`, `.epris/agent/findings.md`). \
       - Execute step-by-step and mark each done as `- [x]` immediately when completed. \
       - Keep this file updated throughout the run so the GUI can show progress. \
       - Do not ask for approval on the plan. \
    \
    The current working directory is the workspace root.";

pub const UI_AGENT_PROMPT_PREFIX: &str = "You are acting as Epris UI-Agent.\n\
\n\
Goal: Expose/edit tunable props and keep the UI contract consistent.\n\
\n\
Runtime reality (critical):\n\
- The end user is in GUI only (preview + handles/controls + Projects + Settings).\n\
- The user cannot answer CLI questions, file-path questions, consent prompts, or plan checkpoints.\n\
- Do not ask for confirmation mid-run. Complete edits and validation in one run.\n\
\n\
Plan tracking (required):\n\
- Create or replace `.epris/agent/active-plan.md` before edits.\n\
- Use checklist steps as `- [ ] ...` and mark them `- [x]` as each step completes.\n\
- Keep `active-plan.md` structural and user-facing only (what appears/behaves), no low-level coding details.\n\
- Store technical details in internal planning files under `.epris/agent/` (e.g. `.epris/agent/task_plan.md`, `.epris/agent/progress.md`, `.epris/agent/findings.md`).\n\
- Keep the file current during the run so GUI can reflect progress.\n\
- Do not request human approval for the plan.\n\
\n\
Hard requirements (atomic): You MUST update all three files together:\n\
1) src/Composition.tsx (logic uses props)\n\
2) src/epris-controls.json (control definitions)\n\
3) src/epris-props.json (current saved values)\n\
\n\
If any file is missing or inconsistent, the UI may crash and validation will fail.\n\
\n\
Rules:\n\
- All control ids MUST be valid JS identifiers: ^[A-Za-z_$][A-Za-z0-9_$]*$.\n\
- If a control belongs to a scanned on-screen object, set control.objectId to the matching <EprisGroup id=\"...\">.\n\
- Prefer creating a single new prop to control multiple things (e.g. themeSize, particleSpeed) and distribute inside Composition.tsx.\n\
- The controls panel UI/widgets are prebuilt by Epris. Never add custom controls UI components; only update the JSON contract + Composition usage.\n\
- `ui` must match `type`: number->slider|input, color->color, select->select, boolean->toggle, text->text|textarea.\n\
- Keep changes within src/**.\n\
- Do not modify package.json / pnpm-lock.yaml.\n\
\n\
Optional (for V1 scan): When you create a major on-screen object, prefer wrapping it with <EprisGroup id=... label=... kind=...> so the app can list it in the Objects panel.\n";

const CLI_PROMPT_CONTEXT_PREFIX: &str = "Epris runtime context (critical):\n\
- The user is using a GUI, not a terminal.\n\
- The user only sees preview, exposed handles/controls, Projects, and Settings.\n\
- The user cannot respond to CLI questions, plan approvals, or filesystem-related prompts.\n\
- Execute end-to-end without pausing for consent.\n\
- Plan tracking is mandatory: create/update `.epris/agent/active-plan.md` as a checkbox checklist and mark steps complete during execution.\n\
- `active-plan.md` must stay non-technical and user-facing; put implementation details in `.epris/agent/task_plan.md`, `.epris/agent/progress.md`, `.epris/agent/findings.md`.\n\
- Epris controls UI is prebuilt by the host app; do not create custom settings UI code. Use `src/epris-controls.json` + `src/epris-props.json` for exposed props.\n\
- If a required npm dependency is missing, output `DEPENDENCY_REQUEST: <pkg>` and stop.";

const RUNTIME_CONTEXT_REMINDER: &str = "Epris runtime reminder:\n\
- Treat this as a non-interactive run; do not wait for user replies mid-run.\n\
- The user cannot see CLI/filesystem internals; provide outcomes through code changes.\n\
- Prefer stable edits that keep preview working in the GUI.\n\
- Keep `.epris/agent/active-plan.md` updated with step checkboxes as work progresses, with structural user-facing wording only.";

const GEMINI_MAX_AUTO_FIX_ATTEMPTS: usize = 2;
const GEMINI_MAX_NOOP_RETRY_ATTEMPTS: usize = 2;

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
        Self {
            process: None,
            port: 4096,
            session_id: None,
        }
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
    let _ = app_handle.emit(
        "prompt-progress",
        PromptProgress {
            percent: 0.0,
            step: "Canceled".into(),
        },
    );
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
    let _ = writeln!(
        log_file,
        "\n--- OpenCode Server Log [{}] ---",
        chrono::Utc::now()
    );

    let mut cmd = utils::create_shell_command(
        "opencode",
        &[
            "serve",
            "--hostname",
            "127.0.0.1",
            "--port",
            &port.to_string(),
        ],
    );
    cmd.current_dir(&workspace_path);

    // Ensure the server can find our local toolchain/bin (opencode may not be on system PATH).
    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let toolchain_bin = app_dir.join("toolchain").join("bin");
    let path_sep = if cfg!(target_os = "windows") {
        ";"
    } else {
        ":"
    };
    let current_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!(
        "{}{}{}",
        toolchain_bin.to_string_lossy(),
        path_sep,
        current_path
    );
    cmd.env("PATH", new_path);

    // Reduce unreadable ANSI escape sequences in opencode.log.
    cmd.env("NO_COLOR", "1");
    cmd.env("FORCE_COLOR", "0");
    cmd.env("TERM", "dumb");

    cmd.stdout(Stdio::from(
        log_file.try_clone().map_err(|e| e.to_string())?,
    ));
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
    let _ = crate::agent_plan::ensure_plan_directory(&workspace_path);
    let _plan_poller =
        crate::agent_plan::start_plan_poller(app_handle.clone(), workspace_path.clone());

    // If provider is Gemini, route exclusively to run_provider_cli
    if provider == "gemini" {
        let app_data_dir = app_handle
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?;

        // Step 1: Pre-flight backup
        let _ = app_handle.emit(
            "prompt-progress",
            PromptProgress {
                percent: 10.0,
                step: "Creating pre-flight backup".into(),
            },
        );
        println!("[Epris] Step 1/5: Creating pre-flight backup (Gemini)...");
        snapshot::create_backup(&workspace_path, "pre_flight")?;
        let ai_started_at = std::time::SystemTime::now();

        // Step 2: Run CLI
        let _ = app_handle.emit(
            "prompt-progress",
            PromptProgress {
                percent: 20.0,
                step: "AI Generating Animation Code".into(),
            },
        );
        println!("[Epris] Step 2/5: Running Gemini CLI...");
        let start = std::time::Instant::now();
        let cli_prompt = format!(
            "{}\n\nUSER_REQUEST:\n{}",
            CLI_PROMPT_CONTEXT_PREFIX, &prompt
        );
        let cli_res = crate::provider::run_provider_cli(
            &workspace_path,
            "gemini",
            &cli_prompt,
            Some(model.clone()),
            app_data_dir.clone(),
        )
        .await;
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
        let _ = app_handle.emit(
            "prompt-progress",
            PromptProgress {
                percent: 75.0,
                step: "Waiting for file system to settle".into(),
            },
        );
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
        let _ = app_handle.emit(
            "prompt-progress",
            PromptProgress {
                percent: 85.0,
                step: "Running security checks".into(),
            },
        );
        println!("[Epris] Step 4/5: Running security check...");
        let backup_path = Path::new(&workspace_path).join(".epris/backups/pre_flight");
        let forbidden_files = utils::check_forbidden_files(&workspace_path, &backup_path)?;

        if !forbidden_files.is_empty() {
            // ... handles same as below ...
            // Refactoring common logic would be good but for now duplication is safer
            println!(
                "[Epris] 🔒 SECURITY VIOLATION: AI modified forbidden files: {:?}",
                forbidden_files
            );
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
        let audit =
            sandbox::audit_workspace_after_ai_run(Path::new(&workspace_path), ai_started_at)?;
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

        // Avoid false "success" runs where Gemini exits after only reading files / planning.
        // We only count src/public changes as meaningful output for preview.
        let mut has_code_changes = detect_workspace_code_changes(&workspace_path, &backup_path)?;
        let mut noop_retry_attempts_used = 0usize;
        while !has_code_changes && noop_retry_attempts_used < GEMINI_MAX_NOOP_RETRY_ATTEMPTS {
            noop_retry_attempts_used += 1;
            let diagnostic = summarize_gemini_noop_diagnostic(&workspace_path);
            let _ = app_handle.emit(
                "prompt-progress",
                PromptProgress {
                    percent: 88.0,
                    step: format!(
                        "No-op recovery retry ({}/{})",
                        noop_retry_attempts_used, GEMINI_MAX_NOOP_RETRY_ATTEMPTS
                    ),
                },
            );
            println!(
                "[Epris] No-op recovery round {}/{}: previous Gemini run made no src/public changes...",
                noop_retry_attempts_used, GEMINI_MAX_NOOP_RETRY_ATTEMPTS
            );
            let retry_prompt = build_gemini_noop_retry_prompt(
                &prompt,
                &diagnostic,
                noop_retry_attempts_used,
                GEMINI_MAX_NOOP_RETRY_ATTEMPTS,
            );
            let retry_start = std::time::Instant::now();
            let retry_res = crate::provider::run_provider_cli(
                &workspace_path,
                "gemini",
                &retry_prompt,
                Some(model.clone()),
                app_data_dir.clone(),
            )
            .await;
            if let Err(e) = retry_res {
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

            let _ = log_ai_performance(
                &workspace_path,
                &format!(
                    "NOOP_RECOVERY_ATTEMPT {} | {}",
                    noop_retry_attempts_used,
                    prompt.replace('\n', " ")
                ),
                retry_start.elapsed().as_secs_f64(),
            );

            tokio::time::sleep(tokio::time::Duration::from_millis(700)).await;
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

            // Re-run security checks after each no-op recovery round.
            let forbidden_files = utils::check_forbidden_files(&workspace_path, &backup_path)?;
            if !forbidden_files.is_empty() {
                println!(
                    "[Epris] 🔒 SECURITY VIOLATION: AI modified forbidden files during no-op recovery: {:?}",
                    forbidden_files
                );
                snapshot::restore_backup(&workspace_path, "pre_flight")?;
                return Ok(PromptResponse {
                    success: false,
                    message:
                        "🔒 Security violation during no-op recovery: Files protected. Reverted."
                            .into(),
                    gate_result: None,
                    snapshot_id: None,
                    canceled: false,
                    validation_skipped: false,
                });
            }

            let audit =
                sandbox::audit_workspace_after_ai_run(Path::new(&workspace_path), ai_started_at)?;
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

            has_code_changes = detect_workspace_code_changes(&workspace_path, &backup_path)?;
        }

        if !has_code_changes {
            let diagnostic = summarize_gemini_noop_diagnostic(&workspace_path);
            return Err(format!(
                "Gemini run completed but produced no changes in src/public after {} no-op recovery attempt(s). \
                     This is usually a tool-policy or prompt-parsing no-op. {}",
                noop_retry_attempts_used, diagnostic
            ));
        }

        // Step 5: Fast validation + bounded auto-fix loop (cheap checks only).
        // Run smoke checks only once after fast validation passes.
        let _ = app_handle.emit(
            "prompt-progress",
            PromptProgress {
                percent: 95.0,
                step: "Fast validation (contract + typecheck)".into(),
            },
        );
        println!("[Epris] Step 5/5: Running fast validation...");
        let parent_id =
            snapshot::get_dag_head(workspace_path.clone()).unwrap_or_else(|_| "root".into());

        let mut fast_result =
            gate::run_gate_with_mode(workspace_path.clone(), GateMode::Fast, Some(&app_handle))
                .await?;
        fast_result.attempts = 1;

        let mut auto_fix_attempts_used = 0usize;
        while !fast_result.passed
            && auto_fix_attempts_used < GEMINI_MAX_AUTO_FIX_ATTEMPTS
            && !fast_result
                .error_output
                .trim()
                .starts_with("DEPENDENCY_REQUEST:")
        {
            if fast_result.error_output == "Gate canceled" && canceled() {
                return Ok(PromptResponse {
                    success: false,
                    message: "Canceled".into(),
                    gate_result: None,
                    snapshot_id: None,
                    canceled: true,
                    validation_skipped: false,
                });
            }

            auto_fix_attempts_used += 1;
            let _ = app_handle.emit(
                "prompt-progress",
                PromptProgress {
                    percent: 92.0,
                    step: format!(
                        "Auto-fixing validation errors ({}/{})",
                        auto_fix_attempts_used, GEMINI_MAX_AUTO_FIX_ATTEMPTS
                    ),
                },
            );
            println!(
                "[Epris] Auto-fix round {}/{}: asking Gemini to fix validation errors...",
                auto_fix_attempts_used, GEMINI_MAX_AUTO_FIX_ATTEMPTS
            );

            let fix_prompt = build_gemini_autofix_prompt(
                &prompt,
                &fast_result,
                auto_fix_attempts_used,
                GEMINI_MAX_AUTO_FIX_ATTEMPTS,
            );
            let fix_start = std::time::Instant::now();
            let fix_res = crate::provider::run_provider_cli(
                &workspace_path,
                "gemini",
                &fix_prompt,
                Some(model.clone()),
                app_data_dir.clone(),
            )
            .await;
            if let Err(e) = fix_res {
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

            let _ = log_ai_performance(
                &workspace_path,
                &format!(
                    "AUTO_FIX_ATTEMPT {} | {}",
                    auto_fix_attempts_used,
                    prompt.replace('\n', " ")
                ),
                fix_start.elapsed().as_secs_f64(),
            );

            tokio::time::sleep(tokio::time::Duration::from_millis(700)).await;
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

            // Re-run security checks after each auto-fix round.
            let forbidden_files = utils::check_forbidden_files(&workspace_path, &backup_path)?;
            if !forbidden_files.is_empty() {
                println!(
                    "[Epris] 🔒 SECURITY VIOLATION: AI modified forbidden files during auto-fix: {:?}",
                    forbidden_files
                );
                snapshot::restore_backup(&workspace_path, "pre_flight")?;
                return Ok(PromptResponse {
                    success: false,
                    message: "🔒 Security violation during auto-fix: Files protected. Reverted."
                        .into(),
                    gate_result: None,
                    snapshot_id: None,
                    canceled: false,
                    validation_skipped: false,
                });
            }

            let audit =
                sandbox::audit_workspace_after_ai_run(Path::new(&workspace_path), ai_started_at)?;
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

            let _ = app_handle.emit(
                "prompt-progress",
                PromptProgress {
                    percent: 94.0,
                    step: "Re-running fast validation".into(),
                },
            );
            fast_result =
                gate::run_gate_with_mode(workspace_path.clone(), GateMode::Fast, Some(&app_handle))
                    .await?;
            fast_result.attempts = (auto_fix_attempts_used + 1) as i32;
        }

        if fast_result.error_output == "Gate canceled" && canceled() {
            return Ok(PromptResponse {
                success: false,
                message: "Canceled".into(),
                gate_result: None,
                snapshot_id: None,
                canceled: true,
                validation_skipped: false,
            });
        }

        if !fast_result.passed {
            let draft_snapshot_id = snapshot::auto_save_snapshot(
                workspace_path.clone(),
                prompt.clone(),
                parent_id.clone(),
                "gemini".into(),
            )
            .ok();
            if let Some(id) = draft_snapshot_id.clone() {
                let _ = snapshot::update_dag_snapshot_gate_result(
                    workspace_path.clone(),
                    id,
                    fast_result.clone(),
                );
            }
            return Ok(PromptResponse {
                success: false,
                message: format!(
                    "Validation failed after {} auto-fix attempt(s).",
                    auto_fix_attempts_used
                ),
                gate_result: Some(fast_result),
                snapshot_id: draft_snapshot_id,
                canceled: false,
                validation_skipped: false,
            });
        }

        // Fast checks passed. Run slower smoke validation once.
        let _ = app_handle.emit(
            "prompt-progress",
            PromptProgress {
                percent: 95.0,
                step: "Running smoke validation".into(),
            },
        );
        let gate_result = gate_loop(workspace_path.clone(), app_handle.clone()).await?;
        if gate_result.error_output == "Gate canceled" && canceled() {
            return Ok(PromptResponse {
                success: false,
                message: "Canceled".into(),
                gate_result: None,
                snapshot_id: None,
                canceled: true,
                validation_skipped: false,
            });
        }

        let draft_snapshot_id = snapshot::auto_save_snapshot(
            workspace_path.clone(),
            prompt.clone(),
            parent_id,
            "gemini".into(),
        )
        .ok();
        if let Some(id) = draft_snapshot_id.clone() {
            let _ = snapshot::update_dag_snapshot_gate_result(
                workspace_path.clone(),
                id,
                gate_result.clone(),
            );
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
            let res = match client
                .post(format!("{}/session", base_url))
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

            let body: serde_json::Value = res
                .json()
                .await
                .map_err(|e| format!("Failed to parse session response: {}", e))?;
            let new_id = body["id"]
                .as_str()
                .ok_or("OpenCode response missing id")?
                .to_string();

            {
                let mut oc = state.lock().map_err(|e| e.to_string())?;
                oc.session_id = Some(new_id.clone());
            }
            println!("[Epris] OpenCode session created: {}", new_id);
            new_id
        }
    };

    println!("[Epris] Step 1/5: Creating pre-flight backup...");
    let _ = app_handle.emit(
        "prompt-progress",
        PromptProgress {
            percent: 10.0,
            step: "Creating pre-flight backup".into(),
        },
    );
    snapshot::create_backup(&workspace_path, "pre_flight")?;
    let ai_started_at = std::time::SystemTime::now();

    println!("[Epris] Step 2/5: Sending prompt to AI: \"{}\"", prompt);
    let _ = app_handle.emit(
        "prompt-progress",
        PromptProgress {
            percent: 20.0,
            step: "AI Generating Animation Code".into(),
        },
    );

    let remotion_rules =
        std::fs::read_to_string(std::path::Path::new(&workspace_path).join("REMOTION.md"))
            .unwrap_or_else(|_| OPENCODE_SYSTEM_PROMPT.to_string());

    let prompt_start = std::time::Instant::now();
    let system_prompt = format!(
        "{}\n\n{}\n\nInstalled npm packages:\n{}\n\nThe Remotion workspace is located at: {}. You MUST read src/Composition.tsx and src/Root.tsx.",
        remotion_rules,
        RUNTIME_CONTEXT_REMINDER,
        describe_installed_packages(&workspace_path),
        workspace_path
    );

    let res = match client
        .post(format!("{}/session/{}/message", base_url, sid))
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

    let body: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse response JSON: {}", e))?;
    let duration = prompt_start.elapsed().as_secs_f64();
    println!("[Epris] Step 2/5: AI response received in {:.2}s", duration);

    let _ = log_ai_performance(&workspace_path, &prompt, duration);

    let message_text = body["parts"]
        .as_array()
        .and_then(|parts| {
            parts
                .iter()
                .find(|p| p["type"] == "text")
                .and_then(|p| p["text"].as_str())
        })
        .unwrap_or("Check workspace for changes");

    println!("[Epris] Step 3/5: Waiting 3s for file system to settle...");
    let _ = app_handle.emit(
        "prompt-progress",
        PromptProgress {
            percent: 75.0,
            step: "Waiting for file system to settle".into(),
        },
    );
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    println!("[Epris] Step 4/5: Running security check...");
    let _ = app_handle.emit(
        "prompt-progress",
        PromptProgress {
            percent: 85.0,
            step: "Running security checks".into(),
        },
    );
    let backup_path = Path::new(&workspace_path).join(".epris/backups/pre_flight");
    let forbidden_files = utils::check_forbidden_files(&workspace_path, &backup_path)?;

    if !forbidden_files.is_empty() {
        println!(
            "[Epris] 🔒 SECURITY VIOLATION: AI modified forbidden files: {:?}",
            forbidden_files
        );
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

    let draft_snapshot_id = snapshot::auto_save_snapshot(
        workspace_path.clone(),
        prompt.clone(),
        parent_id,
        sid.clone(),
    )
    .ok();

    println!("[Epris] Step 5/5: Entering Gate Validation loop...");
    let _ = app_handle.emit(
        "prompt-progress",
        PromptProgress {
            percent: 95.0,
            step: "Validating animation integrity".into(),
        },
    );
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
        let _ = snapshot::update_dag_snapshot_gate_result(
            workspace_path.clone(),
            id,
            gate_result.clone(),
        );
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

#[tauri::command]
pub async fn send_ui_prompt(
    state: tauri::State<'_, Mutex<OpenCodeState>>,
    workspace_path: String,
    prompt: String,
    provider: String,
    model: String,
    draft_props: Option<serde_json::Value>,
    app_handle: tauri::AppHandle,
) -> Result<PromptResponse, String> {
    let mut full_prompt = String::new();
    full_prompt.push_str(UI_AGENT_PROMPT_PREFIX);
    full_prompt.push_str("\n\nUSER_REQUEST:\n");
    full_prompt.push_str(&prompt);

    if let Some(p) = draft_props {
        if let Ok(text) = serde_json::to_string_pretty(&p) {
            // Prevent extremely large prompts.
            let truncated = if text.len() > 50_000 {
                &text[..50_000]
            } else {
                &text
            };
            full_prompt.push_str(
                "\n\nCURRENT_DRAFT_PROPS (may differ from saved epris-props.json):\n```json\n",
            );
            full_prompt.push_str(truncated);
            full_prompt.push_str("\n```\n");
        }
    }

    send_prompt(
        state,
        workspace_path,
        full_prompt,
        provider,
        model,
        app_handle,
    )
    .await
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
        (
            provider_param.trim().to_string(),
            model_param.trim().to_string(),
        )
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

fn collect_relative_files(root: &Path) -> Result<BTreeSet<String>, String> {
    let mut files = BTreeSet::new();
    if !root.exists() {
        return Ok(files);
    }

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(root)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        files.insert(rel);
    }
    Ok(files)
}

fn dir_changed(workspace_dir: &Path, backup_dir: &Path) -> Result<bool, String> {
    let ws_files = collect_relative_files(workspace_dir)?;
    let bk_files = collect_relative_files(backup_dir)?;
    if ws_files != bk_files {
        return Ok(true);
    }

    for rel in ws_files {
        let ws = workspace_dir.join(&rel);
        let bk = backup_dir.join(&rel);
        let ws_bytes = std::fs::read(&ws).map_err(|e| e.to_string())?;
        let bk_bytes = std::fs::read(&bk).map_err(|e| e.to_string())?;
        if ws_bytes != bk_bytes {
            return Ok(true);
        }
    }

    Ok(false)
}

fn detect_workspace_code_changes(workspace_path: &str, backup_path: &Path) -> Result<bool, String> {
    for dir in crate::utils::SNAPSHOT_WHITELIST {
        let ws_dir = Path::new(workspace_path).join(dir);
        let bk_dir = backup_path.join(dir);
        if dir_changed(&ws_dir, &bk_dir)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn summarize_gemini_noop_diagnostic(workspace_path: &str) -> String {
    let log_path = Path::new(workspace_path).join("logs").join("gemini.log");
    let text = match std::fs::read_to_string(&log_path) {
        Ok(t) => t,
        Err(_) => {
            return "Could not read logs/gemini.log for diagnostics.".to_string();
        }
    };

    let mut hints: Vec<&str> = Vec::new();
    if text.contains("Tool execution denied by policy") {
        hints.push("Detected tool policy denial in logs");
    }
    if text.contains("run_shell_command\" not found in registry")
        || text.contains("run_shell_command: Tool \"run_shell_command\" not found")
    {
        hints.push("Detected missing run_shell_command tool in registry");
    }
    if text.contains("activate_skill") && text.contains("denied by policy") {
        hints.push("Detected skill activation denied by policy");
    }
    let has_activate_skill = text.contains("toolCall.name: activate_skill");
    let has_followup_tool = text.contains("toolCall.name: read_file")
        || text.contains("toolCall.name: write_file")
        || text.contains("toolCall.name: replace")
        || text.contains("toolCall.name: run_shell_command");
    if has_activate_skill && !has_followup_tool {
        hints.push("Detected skill activation without follow-up file/shell tool execution");
    }
    if text.contains("You have exhausted your capacity on this model") {
        hints.push("Detected transient model capacity throttling");
    }
    if text.contains("Command parsing failed") && text.contains("Falling back to ASK_USER") {
        hints.push("Detected shell command parsing fallback to ASK_USER");
    }

    if hints.is_empty() {
        "Review logs/gemini.log for details.".to_string()
    } else {
        format!(
            "Likely cause(s): {}. Review logs/gemini.log.",
            hints.join("; ")
        )
    }
}

fn build_gemini_noop_retry_prompt(
    user_prompt: &str,
    diagnostic: &str,
    attempt: usize,
    max_attempts: usize,
) -> String {
    format!(
        "{context}\n\n{reminder}\n\n\
Original user request:\n{user_prompt}\n\n\
The previous run exited without producing file changes under src/** or public/**.\n\
No-op recovery round {attempt}/{max_attempts}:\n\
- Execute actual edits now using tools; do not stop at planning text.\n\
- Write concrete changes under src/** or public/**.\n\
- Do not ask for confirmation.\n\
- If blocked by a missing npm package, output DEPENDENCY_REQUEST: <pkg> and stop.\n\n\
Previous-run diagnostic:\n{diagnostic}\n",
        context = CLI_PROMPT_CONTEXT_PREFIX,
        reminder = RUNTIME_CONTEXT_REMINDER,
        user_prompt = user_prompt,
        attempt = attempt,
        max_attempts = max_attempts,
        diagnostic = diagnostic
    )
}

fn build_gemini_autofix_prompt(
    user_prompt: &str,
    gate_result: &GateResult,
    attempt: usize,
    max_attempts: usize,
) -> String {
    let errors = gate_result
        .error_output
        .chars()
        .take(4_000)
        .collect::<String>();
    format!(
        "{context}\n\n{reminder}\n\n\
Original user request:\n{user_prompt}\n\n\
The current workspace failed automated validation.\n\
Auto-fix round {attempt}/{max_attempts}: fix ONLY the reported validation errors while preserving the requested behavior.\n\
Do not refactor unrelated code.\n\n\
Validation errors:\n{errors}\n",
        context = CLI_PROMPT_CONTEXT_PREFIX,
        reminder = RUNTIME_CONTEXT_REMINDER,
        user_prompt = user_prompt,
        attempt = attempt,
        max_attempts = max_attempts,
        errors = errors
    )
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

    let res = client
        .post(format!("{}/session/{}/message", base_url, session_id))
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
    let mut result = gate::run_gate_with_mode(
        workspace_path.clone(),
        GateMode::Balanced,
        Some(&app_handle),
    )
    .await?;
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
