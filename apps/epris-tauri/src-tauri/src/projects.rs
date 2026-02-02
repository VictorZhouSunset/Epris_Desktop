use crate::environment::EnvironmentManager;
use crate::skills::SkillsManager;
use crate::state_manager::{ProjectState, StateManager};
use crate::utils;
use crate::workspace;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::Manager;

#[cfg(any(test, not(debug_assertions)))]
use include_dir::{include_dir, Dir};

#[cfg(any(test, not(debug_assertions)))]
static WORKSPACE_TEMPLATE_EMBEDDED: Dir<'static> =
    include_dir!("$CARGO_MANIFEST_DIR/../../../workspace-template");

#[cfg(any(test, not(debug_assertions)))]
fn extract_embedded_template_to(dst_root: &Path, version: &str) -> Result<(), String> {
    if dst_root.exists() {
        let _ = std::fs::remove_dir_all(dst_root);
    }
    std::fs::create_dir_all(dst_root).map_err(|e| e.to_string())?;

    for file in WORKSPACE_TEMPLATE_EMBEDDED.files() {
        let rel = file.path();
        let dst = dst_root.join(rel);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&dst, file.contents()).map_err(|e| e.to_string())?;
    }

    std::fs::write(dst_root.join(".epris-template-version"), format!("{}\n", version))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(any(test, not(debug_assertions)))]
fn ensure_embedded_template_dir(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let template_dir = app_dir.join("workspace-template");
    let version_file = template_dir.join(".epris-template-version");
    let current_version = app_handle.package_info().version.to_string();

    let mut needs_extract = true;
    if let Ok(existing) = std::fs::read_to_string(&version_file) {
        if existing.trim() == current_version && template_dir.join("package.json").exists() {
            needs_extract = false;
        }
    }

    if !needs_extract {
        return Ok(template_dir);
    }

    extract_embedded_template_to(&template_dir, &current_version)?;
    Ok(template_dir)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub provider: String,
    pub created_at: Option<String>,
    pub last_opened_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectsOverview {
    pub projects_root: Option<String>,
    pub active_project_id: Option<String>,
    pub projects: Vec<ProjectInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActiveProjectConfig {
    pub path: Option<String>,
    pub exists: bool,
}

fn default_projects_root(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("projects"))
}

fn to_info(p: &ProjectState) -> ProjectInfo {
    ProjectInfo {
        id: p.id.clone(),
        name: p.name.clone(),
        path: p.path.clone(),
        provider: p.provider.clone(),
        created_at: p.created_at.clone(),
        last_opened_at: p.last_opened_at.clone(),
    }
}

fn load_overview(state_mgr: &StateManager) -> ProjectsOverview {
    let state = state_mgr.read();
    let mut projects: Vec<ProjectInfo> = state.projects.values().map(to_info).collect();
    projects.sort_by(|a, b| b.last_opened_at.cmp(&a.last_opened_at).then_with(|| b.created_at.cmp(&a.created_at)));

    ProjectsOverview {
        projects_root: state.projects_root,
        active_project_id: state.active_project_id,
        projects,
    }
}

fn resolve_template_dir(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    #[cfg(debug_assertions)]
    {
        let _ = app_handle;
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        let project_root = utils::find_project_root(Path::new(&manifest_dir), "workspace-template")
            .ok_or_else(|| format!("Could not find 'workspace-template' anchor by searching upwards from {:?}", manifest_dir))?;
        return Ok(project_root.join("workspace-template"));
    }

    #[cfg(not(debug_assertions))]
    {
        let template = app_handle
            .path()
            .resolve("workspace-template", tauri::path::BaseDirectory::Resource)
            .map_err(|e| format!("Failed to resolve resource 'workspace-template': {}", e))?;
        if template.exists() && template.is_dir() {
            return Ok(template);
        }

        let resource_dir = app_handle
            .path()
            .resource_dir()
            .map_err(|e| format!("Failed to get resource_dir: {}", e))?;

        if let Some(found) = find_template_dir_in_resource_dir(&resource_dir) {
            return Ok(found);
        }

        match ensure_embedded_template_dir(app_handle) {
            Ok(extracted) => Ok(extracted),
            Err(extract_err) => Err(format!(
                "Template directory not found. Resolved path: {}. resource_dir: {}. embedded_extract_error: {}",
                template.to_string_lossy(),
                resource_dir.to_string_lossy(),
                extract_err
            )),
        }
    }
}

#[allow(dead_code)]
fn resource_dir_looks_like_workspace_template(resource_dir: &Path) -> bool {
    let pkg = resource_dir.join("package.json");
    if !pkg.exists() {
        return false;
    }
    let content = std::fs::read_to_string(pkg).unwrap_or_default();
    content.contains("\"name\"") && content.contains("epris-workspace-template")
}

#[allow(dead_code)]
fn find_template_dir_in_resource_dir(resource_dir: &Path) -> Option<PathBuf> {
    // Some bundlers place the directory as-is (resource_dir/workspace-template).
    // Others may copy directory contents into resource_dir directly.
    if resource_dir_looks_like_workspace_template(resource_dir) {
        return Some(resource_dir.to_path_buf());
    }

    let entries = std::fs::read_dir(resource_dir).ok()?;
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        if resource_dir_looks_like_workspace_template(&p) {
            return Some(p);
        }
    }
    None
}

fn ensure_public_assets_dir(project_path: &Path) -> Result<(), String> {
    let assets = project_path.join("public").join("assets");
    std::fs::create_dir_all(&assets).map_err(|e| e.to_string())
}

fn ensure_logs_dir(project_path: &Path) -> Result<(), String> {
    let logs = project_path.join("logs");
    std::fs::create_dir_all(&logs).map_err(|e| e.to_string())
}

fn sanitize_folder_name(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if c == ' ' || c == '-' || c == '_' {
            out.push('-');
        }
    }
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    out.trim_matches('-').to_string()
}

fn move_dir_robust(src: &Path, dst: &Path) -> Result<(), String> {
    if dst.exists() {
        return Err(format!("Destination already exists: {}", dst.to_string_lossy()));
    }
    if std::fs::rename(src, dst).is_ok() {
        return Ok(());
    }

    std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    let mut options = fs_extra::dir::CopyOptions::new();
    options.copy_inside = true;
    fs_extra::dir::copy(src, dst, &options).map_err(|e| e.to_string())?;
    std::fs::remove_dir_all(src).map_err(|e| e.to_string())?;
    Ok(())
}

fn flatten_if_nested_workspace_template(project_dir: &Path) -> Result<(), String> {
    let pkg = project_dir.join("package.json");
    if pkg.exists() {
        return Ok(());
    }

    let nested = project_dir.join("workspace-template");
    let nested_pkg = nested.join("package.json");
    if !nested_pkg.exists() {
        return Ok(());
    }

    // Move children up one level.
    for entry in std::fs::read_dir(&nested).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let src = entry.path();
        let name = entry.file_name();
        let dst = project_dir.join(name);

        if dst.exists() {
            return Err(format!(
                "Failed to flatten nested workspace-template: destination already exists: {}",
                dst.to_string_lossy()
            ));
        }

        let ft = entry.file_type().map_err(|e| e.to_string())?;
        if ft.is_dir() {
            move_dir_robust(&src, &dst)?;
        } else {
            std::fs::copy(&src, &dst).map_err(|e| e.to_string())?;
            let _ = std::fs::remove_file(&src);
        }
    }

    let _ = std::fs::remove_dir_all(&nested);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn detects_template_when_package_json_is_at_resource_root() {
        let tmp = tempdir().unwrap();
        std::fs::write(
            tmp.path().join("package.json"),
            r#"{ "name": "epris-workspace-template" }"#,
        )
        .unwrap();

        let found = find_template_dir_in_resource_dir(tmp.path()).unwrap();
        assert_eq!(found, tmp.path());
    }

    #[test]
    fn detects_template_when_nested_in_workspace_template_dir() {
        let tmp = tempdir().unwrap();
        let nested = tmp.path().join("workspace-template");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(
            nested.join("package.json"),
            r#"{ "name": "epris-workspace-template" }"#,
        )
        .unwrap();

        let found = find_template_dir_in_resource_dir(tmp.path()).unwrap();
        assert_eq!(found, nested);
    }

    #[test]
    fn can_extract_embedded_template_to_app_data_style_dir() {
        let tmp = tempdir().unwrap();
        let dst = tmp.path().join("workspace-template");
        extract_embedded_template_to(&dst, "0.0.0-test").unwrap();
        assert!(dst.join("package.json").exists());
        assert!(dst.join(".epris-template-version").exists());
    }
}

#[tauri::command]
pub fn get_projects_overview(app_handle: tauri::AppHandle) -> Result<ProjectsOverview, String> {
    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let state_mgr = StateManager::new(app_dir);
    Ok(load_overview(&state_mgr))
}

#[tauri::command]
pub fn get_active_project_config(app_handle: tauri::AppHandle) -> Result<ActiveProjectConfig, String> {
    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let state_mgr = StateManager::new(app_dir);
    let state = state_mgr.read();
    let Some(active_id) = state.active_project_id else {
        return Ok(ActiveProjectConfig {
            path: None,
            exists: false,
        });
    };
    let Some(p) = state.projects.get(&active_id) else {
        return Ok(ActiveProjectConfig {
            path: None,
            exists: false,
        });
    };
    let exists = Path::new(&p.path).exists();
    Ok(ActiveProjectConfig {
        path: Some(p.path.clone()),
        exists,
    })
}

#[tauri::command]
pub fn set_projects_root(
    app_handle: tauri::AppHandle,
    new_root: String,
    move_existing: bool,
) -> Result<ProjectsOverview, String> {
    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let state_mgr = StateManager::new(app_dir);

    let new_root_path = PathBuf::from(&new_root);
    std::fs::create_dir_all(&new_root_path).map_err(|e| e.to_string())?;

    if !move_existing {
        let _ = state_mgr.update(|s| {
            s.projects_root = Some(new_root.clone());
        })?;
        return Ok(load_overview(&state_mgr));
    }

    let mut state = state_mgr.read();
    let old_root = state.projects_root.clone();
    state.projects_root = Some(new_root.clone());

    for (_id, p) in state.projects.iter_mut() {
        let old_path = PathBuf::from(&p.path);
        if !old_path.exists() {
            continue;
        }
        let folder_name = old_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| p.id.clone());
        let new_path = new_root_path.join(folder_name);
        move_dir_robust(&old_path, &new_path)?;
        p.path = new_path.to_string_lossy().to_string();
    }

    state_mgr.write(&state)?;

    // Best-effort cleanup of old root if it is empty.
    if let Some(old) = old_root {
        let old_path = PathBuf::from(old);
        if old_path.exists() && old_path.read_dir().map(|mut it| it.next().is_none()).unwrap_or(false) {
            let _ = std::fs::remove_dir_all(old_path);
        }
    }

    Ok(load_overview(&state_mgr))
}

#[tauri::command]
pub fn create_project(
    app_handle: tauri::AppHandle,
    projects_root: Option<String>,
    name: Option<String>,
) -> Result<ProjectInfo, String> {
    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let state_mgr = StateManager::new(app_dir.clone());

    let root = match projects_root {
        Some(p) if !p.trim().is_empty() => PathBuf::from(p),
        _ => {
            let state = state_mgr.read();
            if let Some(r) = state.projects_root.clone() {
                PathBuf::from(r)
            } else {
                default_projects_root(&app_handle)?
            }
        }
    };
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;

    let project_id = uuid::Uuid::new_v4().to_string();
    let base_name = name.unwrap_or_else(|| "My Video".to_string());
    let safe_name = sanitize_folder_name(&base_name);
    let folder_name = if safe_name.is_empty() {
        project_id.clone()
    } else {
        format!("{}-{}", safe_name, &project_id[..8])
    };
    let project_dir = root.join(folder_name);

    let template_dir = resolve_template_dir(&app_handle)?;
    // Reuse the existing workspace mirroring logic to ensure we copy the template contents
    // (not a nested folder), and to apply consistent hygiene rules (e.g. remove node_modules).
    workspace::mirror_workspace(&template_dir, &project_dir)?;
    // Defensive: if template ended up nested due to platform/resource path edge cases, flatten it.
    flatten_if_nested_workspace_template(&project_dir)?;
    ensure_public_assets_dir(&project_dir)?;
    ensure_logs_dir(&project_dir)?;

    // Ensure rule/config files exist for both providers.
    crate::provider::ProviderManager::ensure_rule_files(project_dir.to_string_lossy().as_ref())?;

    // If Remotion skills were already installed/cached, project them into this workspace.
    let env_manager = EnvironmentManager::new(app_dir);
    SkillsManager::sync_cached_skills_into_workspace_if_present(project_dir.to_string_lossy().as_ref(), &env_manager)?;

    let now = Utc::now().to_rfc3339();
    let provider = "opencode".to_string();

    let _ = state_mgr.update(|s| {
        s.projects_root = Some(root.to_string_lossy().to_string());
        s.active_project_id = Some(project_id.clone());
        s.projects.insert(
            project_id.clone(),
            ProjectState {
                schema_version: 1,
                id: project_id.clone(),
                name: base_name.clone(),
                path: project_dir.to_string_lossy().to_string(),
                provider: provider.clone(),
                created_at: Some(now.clone()),
                last_opened_at: Some(now.clone()),
            },
        );
    })?;

    Ok(ProjectInfo {
        id: project_id,
        name: base_name,
        path: project_dir.to_string_lossy().to_string(),
        provider,
        created_at: Some(now.clone()),
        last_opened_at: Some(now),
    })
}

#[tauri::command]
pub fn set_active_project(app_handle: tauri::AppHandle, project_id: String) -> Result<(), String> {
    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let state_mgr = StateManager::new(app_dir);
    let now = Utc::now().to_rfc3339();

    let mut state = state_mgr.read();
    if !state.projects.contains_key(&project_id) {
        return Err("Project not found".to_string());
    }
    state.active_project_id = Some(project_id.clone());
    if let Some(p) = state.projects.get_mut(&project_id) {
        p.last_opened_at = Some(now);
    }
    state_mgr.write(&state)?;
    Ok(())
}

#[tauri::command]
pub fn delete_project(app_handle: tauri::AppHandle, project_id: String) -> Result<(), String> {
    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let state_mgr = StateManager::new(app_dir);
    let mut state = state_mgr.read();

    let Some(p) = state.projects.remove(&project_id) else {
        return Err("Project not found".to_string());
    };

    let path = PathBuf::from(&p.path);
    if path.exists() {
        std::fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
    }

    if state.active_project_id.as_deref() == Some(&project_id) {
        state.active_project_id = state.projects.keys().next().cloned();
    }

    state_mgr.write(&state)?;
    Ok(())
}

#[tauri::command]
pub fn rename_project(app_handle: tauri::AppHandle, project_id: String, name: String) -> Result<(), String> {
    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let state_mgr = StateManager::new(app_dir);
    let mut state = state_mgr.read();

    let trimmed = name.trim().to_string();
    if trimmed.is_empty() {
        return Err("Project name cannot be empty".to_string());
    }

    let Some(p) = state.projects.get_mut(&project_id) else {
        return Err("Project not found".to_string());
    };
    p.name = trimmed;
    state_mgr.write(&state)?;
    Ok(())
}

#[tauri::command]
pub fn get_default_projects_root(app_handle: tauri::AppHandle) -> Result<String, String> {
    Ok(default_projects_root(&app_handle)?.to_string_lossy().to_string())
}

#[tauri::command]
pub fn pick_projects_root(initial: Option<String>) -> Result<Option<String>, String> {
    #[cfg(target_os = "windows")]
    {
        let mut selected_path = initial.unwrap_or_default();
        if selected_path.contains('\'') {
            selected_path = selected_path.replace('\'', "''");
        }
        let script = format!(
            "Add-Type -AssemblyName System.Windows.Forms; \
             $d = New-Object System.Windows.Forms.FolderBrowserDialog; \
             if ('{}' -ne '') {{ $d.SelectedPath = '{}' }}; \
             $d.Description = 'Choose a folder for your Epris projects'; \
             $r = $d.ShowDialog(); \
             if ($r -eq [System.Windows.Forms.DialogResult]::OK) {{ Write-Output $d.SelectedPath }}",
            selected_path, selected_path
        );

        let output = utils::create_shell_command("powershell", &["-NoProfile", "-Command", &script])
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            return Err("Failed to open folder picker".to_string());
        }
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if stdout.is_empty() {
            return Ok(None);
        }
        return Ok(Some(stdout));
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = initial;
        Err("Folder picker not implemented for this OS yet".to_string())
    }
}
