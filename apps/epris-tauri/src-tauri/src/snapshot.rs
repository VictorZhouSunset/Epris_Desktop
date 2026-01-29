use serde::{Serialize, Deserialize};
use std::path::Path;
use uuid::Uuid;
use crate::utils;
use crate::GateResult;

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotDAG {
    pub nodes: Vec<SnapshotMetadata>,
    pub edges: Vec<Edge>,
    pub layout: std::collections::HashMap<String, NodePosition>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
}

fn auto_save_checkpoint_internal(
    workspace_path: &str,
    reason: &str,
    force: bool,
) -> Result<Option<String>, String> {
    let dirty = check_unsaved_changes(workspace_path.to_string()).unwrap_or(false);
    if !force && !dirty {
        return Ok(None);
    }

    let parent_id = get_dag_head(workspace_path.to_string()).unwrap_or_else(|_| "root".to_string());
    let sid = Uuid::new_v4().to_string();
    let name = format!("Auto: {}", reason);

    let snapshot_meta = SnapshotMetadata {
        id: sid,
        name,
        description: reason.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        parent_id: Some(parent_id),
        is_manual: true,
        prompt: None,
        session_id: String::new(),
        gate_result: None,
    };

    let snapshot_id = create_snapshot_with_metadata(workspace_path.to_string(), snapshot_meta)?;
    save_dag_head(workspace_path.to_string(), snapshot_id.clone())?;
    Ok(Some(snapshot_id))
}

#[tauri::command]
pub fn auto_save_checkpoint(
    workspace_path: String,
    reason: String,
    force: Option<bool>,
) -> Result<Option<String>, String> {
    auto_save_checkpoint_internal(&workspace_path, &reason, force.unwrap_or(false))
}

pub fn auto_save_checkpoint_best_effort(workspace_path: &str, reason: &str) {
    let _ = auto_save_checkpoint_internal(workspace_path, reason, false);
}

pub fn create_snapshot_with_metadata(
    workspace_path: String,
    snapshot_meta: SnapshotMetadata,
) -> Result<String, String> {
    let snapshot_id = snapshot_meta.id.clone();
    let snapshot_dir = Path::new(&workspace_path)
        .join(".epris/history")
        .join(&snapshot_id);

    // Create snapshot directory
    std::fs::create_dir_all(&snapshot_dir)
        .map_err(|e| format!("Failed to create snapshot dir: {}", e))?;

    // Copy whitelisted directories
    for dir_name in utils::SNAPSHOT_WHITELIST {
        let src_dir = Path::new(&workspace_path).join(dir_name);
        if src_dir.exists() {
            let mut options = fs_extra::dir::CopyOptions::new();
            options.copy_inside = true;
            options.overwrite = true;
            if let Err(e) = fs_extra::dir::copy(
                &src_dir,
                &snapshot_dir,
                &options
            ) {
                return Err(format!("Failed to copy directory {}: {}", dir_name, e));
            }
        }
    }

    // Copy forbidden files (needed for diffing logic in case of violation)
    for file_name in utils::FORBIDDEN_FILES {
        let src_file = Path::new(&workspace_path).join(file_name);
        if src_file.exists() {
            let dst_file = snapshot_dir.join(file_name);
            if let Err(e) = std::fs::copy(&src_file, &dst_file) {
                return Err(format!("Failed to copy forbidden file {} for diffing: {}", file_name, e));
            }
        }
    }

    // Write metadata
    let meta_path = snapshot_dir.join("meta.json");
    let meta_json = serde_json::to_string_pretty(&snapshot_meta)
        .map_err(|e| format!("Failed to serialize metadata: {}", e))?;
    std::fs::write(&meta_path, meta_json)
        .map_err(|e| format!("Failed to write metadata: {}", e))?;

    Ok(snapshot_id)
}

pub fn restore_snapshot(workspace_path: &str, snapshot_id: &str) -> Result<(), String> {
    let snapshot_dir = Path::new(workspace_path)
        .join(".epris/history")
        .join(snapshot_id);

    if !snapshot_dir.exists() {
        return Err(format!("Snapshot {} not found", snapshot_id));
    }

    for dir_name in utils::SNAPSHOT_WHITELIST {
        let target_dir = Path::new(workspace_path).join(dir_name);
        if target_dir.exists() {
            std::fs::remove_dir_all(&target_dir)
                .map_err(|e| format!("Failed to remove {}: {}", dir_name, e))?;
        }
    }

    // Copy snapshot content back
    for dir_name in utils::SNAPSHOT_WHITELIST {
        let src_dir = snapshot_dir.join(dir_name);
        if src_dir.exists() {
            let mut options = fs_extra::dir::CopyOptions::new();
            options.copy_inside = true;
            options.overwrite = true;
            fs_extra::dir::copy(
                &src_dir,
                Path::new(workspace_path),
                &options
            ).map_err(|e| format!("Failed to restore {}: {}", dir_name, e))?;
        }
    }

    Ok(())
}

pub fn create_backup(workspace_path: &str, backup_name: &str) -> Result<(), String> {
    let backup_dir = Path::new(workspace_path).join(".epris/backups").join(backup_name);
    if backup_dir.exists() {
        std::fs::remove_dir_all(&backup_dir).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;

    // Backup whitelist + forbidden files
    let mut files_to_backup = utils::SNAPSHOT_WHITELIST.to_vec();
    files_to_backup.extend_from_slice(utils::FORBIDDEN_FILES);

    for name in files_to_backup {
        let src = Path::new(workspace_path).join(name);
        if src.exists() {
            let mut options = fs_extra::dir::CopyOptions::new();
            options.copy_inside = true;
            options.overwrite = true;
            if src.is_dir() {
                fs_extra::dir::copy(&src, &backup_dir, &options).map_err(|e| e.to_string())?;
            } else {
                std::fs::copy(&src, backup_dir.join(name)).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

pub fn restore_backup(workspace_path: &str, backup_name: &str) -> Result<(), String> {
    let backup_dir = Path::new(workspace_path).join(".epris/backups").join(backup_name);
    if !backup_dir.exists() {
        return Err(format!("Backup {} not found", backup_name));
    }

    // Clear current workspace files
    let mut files_to_restore = utils::SNAPSHOT_WHITELIST.to_vec();
    files_to_restore.extend_from_slice(utils::FORBIDDEN_FILES);

    for name in files_to_restore {
        let target = Path::new(workspace_path).join(name);
        if target.exists() {
            if target.is_dir() {
                std::fs::remove_dir_all(&target).map_err(|e| e.to_string())?;
            } else {
                std::fs::remove_file(&target).map_err(|e| e.to_string())?;
            }
        }
    }

    // Restore from backup
    let mut options = fs_extra::dir::CopyOptions::new();
    options.copy_inside = true;
    options.overwrite = true;
    for entry in std::fs::read_dir(&backup_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        fs_extra::dir::copy(entry.path(), workspace_path, &options).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub fn clear_snapshot_history(workspace_path: String) -> Result<(), String> {
    let history_dir = Path::new(&workspace_path).join(".epris/history");
    if history_dir.exists() {
        std::fs::remove_dir_all(&history_dir).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&history_dir).map_err(|e| e.to_string())?;
    }
    // clear layout
    let layout_path = Path::new(&workspace_path).join(".epris/layout.json");
    if layout_path.exists() {
        let _ = std::fs::remove_file(layout_path);
    }
    save_dag_head(workspace_path, "root".to_string())?;
    Ok(())
}

#[tauri::command]
pub fn checkout_snapshot(
    workspace_path: String,
    snapshot_id: String,
) -> Result<(), String> {
    restore_snapshot(&workspace_path, &snapshot_id)?;
    save_dag_head(workspace_path, snapshot_id)?;
    Ok(())
}

#[tauri::command]
pub fn get_dag_head(workspace_path: String) -> Result<String, String> {
    let head_path = Path::new(&workspace_path).join(".epris/HEAD");
    if !head_path.exists() {
        return Ok("root".to_string());
    }
    std::fs::read_to_string(head_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_dag_head(workspace_path: String, snapshot_id: String) -> Result<(), String> {
    let epris_dir = Path::new(&workspace_path).join(".epris");
    std::fs::create_dir_all(&epris_dir).map_err(|e| e.to_string())?;
    let head_path = epris_dir.join("HEAD");
    std::fs::write(head_path, snapshot_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn auto_save_snapshot(
    workspace_path: String,
    prompt: String,
    parent_id: String,
    session_id: String,
) -> Result<String, String> {
    let sid = Uuid::new_v4().to_string();
    let name = format!("AI: {}", prompt.chars().take(30).collect::<String>());
    
    let snapshot_meta = SnapshotMetadata {
        id: sid,
        name,
        description: prompt.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        parent_id: Some(parent_id),
        is_manual: false,
        prompt: Some(prompt),
        session_id,
        gate_result: None,
    };

    let snapshot_id = create_snapshot_with_metadata(workspace_path.clone(), snapshot_meta)?;
    save_dag_head(workspace_path, snapshot_id.clone())?;
    Ok(snapshot_id)
}

#[tauri::command]
pub fn manual_save_snapshot(
    workspace_path: String,
    name: String,
    description: String,
    parent_id: String,
) -> Result<String, String> {
    let sid = Uuid::new_v4().to_string();
    let snapshot_meta = SnapshotMetadata {
        id: sid,
        name,
        description,
        timestamp: chrono::Utc::now().to_rfc3339(),
        parent_id: Some(parent_id),
        is_manual: true,
        prompt: None,
        session_id: String::new(),
        gate_result: None,
    };

    let snapshot_id = create_snapshot_with_metadata(workspace_path.clone(), snapshot_meta)?;
    save_dag_head(workspace_path, snapshot_id.clone())?;
    Ok(snapshot_id)
}

#[tauri::command]
pub fn delete_snapshot_tree(
    workspace_path: String,
    snapshot_id: String,
) -> Result<Vec<String>, String> {
    let history_dir = Path::new(&workspace_path).join(".epris/history");
    
    // 1. Find all descendants recursively
    let dag = get_snapshot_dag(workspace_path.clone())?;
    let mut descendants = Vec::new();
    find_descendants_recursive(&snapshot_id, &dag.edges, &mut descendants);
    
    // Add the target itself
    descendants.push(snapshot_id.clone());
    
    // 2. Delete directories
    for id in &descendants {
        let snapshot_dir = history_dir.join(id);
        if snapshot_dir.exists() {
            std::fs::remove_dir_all(&snapshot_dir)
                .map_err(|e| format!("Failed to delete snapshot {}: {}", id, e))?;
        }
    }

    // 3. Cleanup layout entries
    let layout_path = Path::new(&workspace_path).join(".epris/layout.json");
    if layout_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&layout_path) {
            if let Ok(mut layout) = serde_json::from_str::<std::collections::HashMap<String, NodePosition>>(&content) {
                let mut changed = false;
                for id in &descendants {
                    if layout.remove(id).is_some() {
                        changed = true;
                    }
                }
                if changed {
                    if let Ok(json) = serde_json::to_string_pretty(&layout) {
                        let _ = std::fs::write(layout_path, json);
                    }
                }
            }
        }
    }

    Ok(descendants)
}

fn find_descendants_recursive(parent_id: &str, edges: &[Edge], result: &mut Vec<String>) {
    for edge in edges {
        if edge.from == parent_id {
            result.push(edge.to.clone());
            find_descendants_recursive(&edge.to, edges, result);
        }
    }
}

pub fn update_dag_snapshot_gate_result(
    workspace_path: String,
    snapshot_id: String,
    gate_result: GateResult,
) -> Result<(), String> {
    let meta_path = Path::new(&workspace_path)
        .join(".epris/history")
        .join(&snapshot_id)
        .join("meta.json");

    if !meta_path.exists() {
        return Err(format!("Metadata for snapshot {} not found", snapshot_id));
    }

    let meta_content = std::fs::read_to_string(&meta_path)
        .map_err(|e| format!("Failed to read metadata: {}", e))?;
    
    let mut meta: SnapshotMetadata = serde_json::from_str(&meta_content)
        .map_err(|e| format!("Failed to parse metadata: {}", e))?;

    meta.gate_result = Some(gate_result);

    let meta_json = serde_json::to_string_pretty(&meta)
        .map_err(|e| format!("Failed to serialize updated metadata: {}", e))?;
    
    std::fs::write(&meta_path, meta_json)
        .map_err(|e| format!("Failed to write updated metadata: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn get_snapshot_dag(workspace_path: String) -> Result<SnapshotDAG, String> {
    let history_dir = Path::new(&workspace_path).join(".epris/history");
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    if history_dir.exists() {
        for entry in std::fs::read_dir(history_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let meta_path = entry.path().join("meta.json");
            
            if meta_path.exists() {
                let meta_content = std::fs::read_to_string(meta_path).map_err(|e| e.to_string())?;
                if let Ok(meta) = serde_json::from_str::<SnapshotMetadata>(&meta_content) {
                    if let Some(ref parent) = meta.parent_id {
                        edges.push(Edge {
                            from: parent.clone(),
                            to: meta.id.clone(),
                        });
                    }
                    nodes.push(meta);
                }
            }
        }
    }

    let layout_path = Path::new(&workspace_path).join(".epris/layout.json");
    let layout: std::collections::HashMap<String, NodePosition> = if layout_path.exists() {
        let content = std::fs::read_to_string(&layout_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        std::collections::HashMap::new()
    };

    Ok(SnapshotDAG { nodes, edges, layout })
}

#[tauri::command]
pub fn save_snapshot_layout(
    workspace_path: String,
    snapshot_id: String,
    x: f64,
    y: f64,
) -> Result<(), String> {
    let layout_path = Path::new(&workspace_path).join(".epris/layout.json");
    let mut layout: std::collections::HashMap<String, NodePosition> = if layout_path.exists() {
        let content = std::fs::read_to_string(&layout_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        std::collections::HashMap::new()
    };

    layout.insert(snapshot_id, NodePosition { x, y });

    let json = serde_json::to_string_pretty(&layout).map_err(|e| e.to_string())?;
    std::fs::write(layout_path, json).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_snapshots(workspace_path: String) -> Result<Vec<SnapshotMetadata>, String> {
    let dag = get_snapshot_dag(workspace_path)?;
    Ok(dag.nodes)
}

#[tauri::command]
pub fn update_snapshot_metadata(
    workspace_path: String,
    snapshot_id: String,
    name: String,
    description: String,
) -> Result<(), String> {
    let meta_path = Path::new(&workspace_path)
        .join(".epris/history")
        .join(&snapshot_id)
        .join("meta.json");

    if !meta_path.exists() {
        return Err(format!("Snapshot {} not found", snapshot_id));
    }

    let meta_content = std::fs::read_to_string(&meta_path).map_err(|e| e.to_string())?;
    let mut meta: SnapshotMetadata = serde_json::from_str(&meta_content).map_err(|e| e.to_string())?;

    meta.name = name;
    meta.description = description;

    let meta_json = serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?;
    std::fs::write(meta_path, meta_json).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_gate_history(workspace_path: String) -> Result<Vec<GateResult>, String> {
    let logs_dir = Path::new(&workspace_path).join("logs");
    let gate_log = logs_dir.join("gate.jsonl");

    if !gate_log.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(gate_log)
        .map_err(|e| format!("Failed to read gate logs: {}", e))?;

    let results = content.lines()
        .filter_map(|line| serde_json::from_str::<GateResult>(line).ok())
        .collect();

    Ok(results)
}

#[tauri::command]
pub fn check_unsaved_changes(workspace_path: String) -> Result<bool, String> {
    let head_id = get_dag_head(workspace_path.clone())?;
    if head_id == "root" {
        // Technically everything is unsaved if we haven't made a first snapshot
        // but we treat "root" as clean if there's nothing in history.
        // Actually, let's just compare with nothing? No.
        return Ok(false);
    }

    let snapshot_dir = Path::new(&workspace_path)
        .join(".epris/history")
        .join(head_id);
    
    if !snapshot_dir.exists() {
        return Ok(false);
    }

    for dir_name in utils::SNAPSHOT_WHITELIST {
        let ws_dir = Path::new(&workspace_path).join(dir_name);
        let ss_dir = snapshot_dir.join(dir_name);
        
        if dir_contents_differ(&ws_dir, &ss_dir)? {
            return Ok(true);
        }
    }

    Ok(false)
}

fn dir_contents_differ(ws_dir: &Path, ss_dir: &Path) -> Result<bool, String> {
    if !ws_dir.exists() || !ss_dir.exists() {
        return Ok(ws_dir.exists() != ss_dir.exists());
    }

    // Rough check: compare filenames and sizes
    let mut ws_files = std::collections::HashMap::new();
    for entry in walkdir::WalkDir::new(ws_dir) {
        let entry = entry.map_err(|e: walkdir::Error| e.to_string())?;
        if entry.file_type().is_file() {
            let rel_path = entry.path().strip_prefix(ws_dir).map_err(|e: std::path::StripPrefixError| e.to_string())?;
            let metadata = entry.metadata().map_err(|e: walkdir::Error| e.to_string())?;
            ws_files.insert(rel_path.to_path_buf(), metadata.len());
        }
    }

    let mut ss_files = std::collections::HashMap::new();
    for entry in walkdir::WalkDir::new(ss_dir) {
        let entry = entry.map_err(|e: walkdir::Error| e.to_string())?;
        if entry.file_type().is_file() {
            let rel_path = entry.path().strip_prefix(ss_dir).map_err(|e: std::path::StripPrefixError| e.to_string())?;
            let metadata = entry.metadata().map_err(|e: walkdir::Error| e.to_string())?;
            ss_files.insert(rel_path.to_path_buf(), metadata.len());
        }
    }

    if ws_files.len() != ss_files.len() {
        return Ok(true);
    }

    for (path, size) in ws_files {
        if let Some(ss_size) = ss_files.get(&path) {
            if size != *ss_size {
                return Ok(true);
            }
            // For V0, size check is enough for "unsaved" hint.
            // Full content hash would be better but expensive.
        } else {
            return Ok(true);
        }
    }

    Ok(false)
}
