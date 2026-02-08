use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Emitter;

pub const PLAN_RELATIVE_PATH: &str = ".epris/agent/active-plan.md";
pub const PLAN_EVENT_NAME: &str = "agent-plan";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentPlanStep {
    pub title: String,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentPlanState {
    pub exists: bool,
    pub path: String,
    pub steps: Vec<AgentPlanStep>,
    pub completed: usize,
    pub total: usize,
    pub updated_at: Option<String>,
}

impl AgentPlanState {
    fn empty(path: &Path) -> Self {
        Self {
            exists: false,
            path: path.to_string_lossy().into_owned(),
            steps: Vec::new(),
            completed: 0,
            total: 0,
            updated_at: None,
        }
    }
}

fn parse_checkbox_line(line: &str) -> Option<AgentPlanStep> {
    let trimmed = line.trim_start();
    let prefixes = [
        ("- [ ]", false),
        ("- [x]", true),
        ("- [X]", true),
        ("* [ ]", false),
        ("* [x]", true),
        ("* [X]", true),
    ];

    for (prefix, done) in prefixes {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            let title = rest.trim();
            if !title.is_empty() {
                return Some(AgentPlanStep {
                    title: title.to_string(),
                    done,
                });
            }
        }
    }
    None
}

pub fn get_plan_path(workspace_path: &str) -> PathBuf {
    Path::new(workspace_path).join(PLAN_RELATIVE_PATH)
}

pub fn ensure_plan_directory(workspace_path: &str) -> Result<PathBuf, String> {
    let path = get_plan_path(workspace_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    Ok(path)
}

pub fn read_agent_plan_state(workspace_path: &str) -> Result<AgentPlanState, String> {
    let path = get_plan_path(workspace_path);
    if !path.exists() {
        return Ok(AgentPlanState::empty(&path));
    }

    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let steps: Vec<AgentPlanStep> = text.lines().filter_map(parse_checkbox_line).collect();
    let completed = steps.iter().filter(|s| s.done).count();
    let updated_at = std::fs::metadata(&path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339());

    Ok(AgentPlanState {
        exists: true,
        path: path.to_string_lossy().into_owned(),
        total: steps.len(),
        completed,
        steps,
        updated_at,
    })
}

pub fn start_plan_poller(app_handle: tauri::AppHandle, workspace_path: String) -> PlanPollerGuard {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_flag = Arc::clone(&stop);
    let app = app_handle.clone();

    tauri::async_runtime::spawn(async move {
        let mut last_sent: Option<AgentPlanState> = None;
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(700));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        // Emit once immediately so UI can show target path before file is created.
        if let Ok(state) = read_agent_plan_state(&workspace_path) {
            let _ = app.emit(PLAN_EVENT_NAME, state.clone());
            last_sent = Some(state);
        }

        loop {
            if stop_flag.load(Ordering::Relaxed) {
                break;
            }
            interval.tick().await;
            if stop_flag.load(Ordering::Relaxed) {
                break;
            }
            if let Ok(state) = read_agent_plan_state(&workspace_path) {
                if last_sent.as_ref() != Some(&state) {
                    let _ = app.emit(PLAN_EVENT_NAME, state.clone());
                    last_sent = Some(state);
                }
            }
        }

        if let Ok(state) = read_agent_plan_state(&workspace_path) {
            if last_sent.as_ref() != Some(&state) {
                let _ = app.emit(PLAN_EVENT_NAME, state);
            }
        }
    });

    PlanPollerGuard { stop }
}

pub struct PlanPollerGuard {
    stop: Arc<AtomicBool>,
}

impl Drop for PlanPollerGuard {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

#[tauri::command]
pub fn get_agent_plan_state(workspace_path: String) -> Result<AgentPlanState, String> {
    read_agent_plan_state(&workspace_path)
}
