use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use base64::Engine;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssetInfo {
    pub name: String,
    pub rel_path: String,
    pub size_bytes: u64,
}

fn assets_dir(workspace_path: &str) -> PathBuf {
    Path::new(workspace_path).join("public").join("assets")
}

fn normalize_filename(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Filename cannot be empty".to_string());
    }
    if name.contains('/') || name.contains('\\') {
        return Err("Invalid filename".to_string());
    }
    Ok(name.to_string())
}

fn normalize_rel_path(rel: &str) -> Result<String, String> {
    let rel = rel.trim().replace('\\', "/");
    if rel.is_empty() {
        return Err("Path cannot be empty".to_string());
    }
    if rel.starts_with('/') || rel.contains("..") {
        return Err("Invalid asset path".to_string());
    }
    if !rel.starts_with("assets/") {
        return Err("Asset path must start with assets/".to_string());
    }
    Ok(rel)
}

#[tauri::command]
pub fn list_assets(workspace_path: String) -> Result<Vec<AssetInfo>, String> {
    let dir = assets_dir(&workspace_path);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut out: Vec<AssetInfo> = Vec::new();
    for entry in walkdir::WalkDir::new(&dir).min_depth(1).max_depth(5) {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().is_file() {
            continue;
        }
        let meta = entry.metadata().map_err(|e| e.to_string())?;
        let rel = entry
            .path()
            .strip_prefix(Path::new(&workspace_path).join("public"))
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        let name = entry.file_name().to_string_lossy().to_string();
        out.push(AssetInfo {
            name,
            rel_path: rel,
            size_bytes: meta.len(),
        });
    }
    out.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    Ok(out)
}

#[tauri::command]
pub fn upload_asset(workspace_path: String, filename: String, bytes_base64: String) -> Result<AssetInfo, String> {
    let filename = normalize_filename(&filename)?;
    let dir = assets_dir(&workspace_path);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let data = base64::engine::general_purpose::STANDARD
        .decode(bytes_base64.as_bytes())
        .map_err(|e| format!("Failed to decode base64: {}", e))?;
    let target = dir.join(&filename);
    std::fs::write(&target, data).map_err(|e| e.to_string())?;

    let meta = std::fs::metadata(&target).map_err(|e| e.to_string())?;
    Ok(AssetInfo {
        name: filename.clone(),
        rel_path: format!("assets/{}", filename),
        size_bytes: meta.len(),
    })
}

#[tauri::command]
pub fn delete_asset(workspace_path: String, rel_path: String) -> Result<(), String> {
    let rel = normalize_rel_path(&rel_path)?;
    let assets_root = assets_dir(&workspace_path);
    let target = assets_root.join(rel.strip_prefix("assets/").unwrap_or(""));
    // Ensure target is inside assets dir
    let canon_root = assets_root.canonicalize().map_err(|e| e.to_string())?;
    let canon_target = target.canonicalize().map_err(|e| e.to_string())?;
    if !canon_target.starts_with(&canon_root) {
        return Err("Invalid asset path".to_string());
    }
    std::fs::remove_file(&canon_target).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn rename_asset(workspace_path: String, rel_path: String, new_name: String) -> Result<AssetInfo, String> {
    let rel = normalize_rel_path(&rel_path)?;
    let new_name = normalize_filename(&new_name)?;

    let assets_root = assets_dir(&workspace_path);
    std::fs::create_dir_all(&assets_root).map_err(|e| e.to_string())?;

    let from = assets_root.join(rel.strip_prefix("assets/").unwrap_or(""));
    let to = assets_root.join(&new_name);

    let canon_root = assets_root.canonicalize().map_err(|e| e.to_string())?;
    let canon_from = from.canonicalize().map_err(|e| e.to_string())?;
    if !canon_from.starts_with(&canon_root) {
        return Err("Invalid asset path".to_string());
    }

    std::fs::rename(&canon_from, &to).map_err(|e| e.to_string())?;
    let meta = std::fs::metadata(&to).map_err(|e| e.to_string())?;
    Ok(AssetInfo {
        name: new_name.clone(),
        rel_path: format!("assets/{}", new_name),
        size_bytes: meta.len(),
    })
}
