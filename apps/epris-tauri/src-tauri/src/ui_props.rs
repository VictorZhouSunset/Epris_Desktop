use serde_json::Value;
use std::path::{Path, PathBuf};

const MAX_PROPS_JSON_BYTES: usize = 256 * 1024;

fn is_ascii_js_identifier(s: &str) -> bool {
    let mut it = s.chars();
    let first = match it.next() {
        Some(c) => c,
        None => return false,
    };
    let first_ok = first == '_' || first == '$' || first.is_ascii_alphabetic();
    if !first_ok {
        return false;
    }
    for c in it {
        if !(c == '_' || c == '$' || c.is_ascii_alphanumeric()) {
            return false;
        }
    }
    true
}

fn workspace_src_dir(workspace_path: &str) -> Result<PathBuf, String> {
    let ws = Path::new(workspace_path);
    let ws_canon = ws
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize workspace: {}", e))?;
    Ok(ws_canon.join("src"))
}

fn read_json_file(path: &Path) -> Result<Value, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

fn write_json_file(path: &Path, value: &Value) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_PROPS_JSON_BYTES {
        return Err(format!(
            "Refusing to write JSON larger than {} bytes",
            MAX_PROPS_JSON_BYTES
        ));
    }
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_epris_controls(workspace_path: String) -> Result<Value, String> {
    let src = workspace_src_dir(&workspace_path)?;
    if !src.exists() {
        return Ok(serde_json::json!({
            "schemaVersion": 1,
            "target": { "kind": "composition", "id": "Main" },
            "controls": [],
        }));
    }
    let path = src.join("epris-controls.json");
    if !path.exists() {
        return Ok(serde_json::json!({
            "schemaVersion": 1,
            "target": { "kind": "composition", "id": "Main" },
            "controls": [],
        }));
    }
    read_json_file(&path)
}

#[tauri::command]
pub fn get_epris_props(workspace_path: String) -> Result<Value, String> {
    let src = workspace_src_dir(&workspace_path)?;
    if !src.exists() {
        return Ok(serde_json::json!({
            "schemaVersion": 1,
            "values": {},
        }));
    }
    let path = src.join("epris-props.json");
    if !path.exists() {
        return Ok(serde_json::json!({
            "schemaVersion": 1,
            "values": {},
        }));
    }
    read_json_file(&path)
}

#[tauri::command]
pub fn set_epris_props(workspace_path: String, values: Value) -> Result<Value, String> {
    let src = workspace_src_dir(&workspace_path)?;
    std::fs::create_dir_all(&src).map_err(|e| e.to_string())?;
    let path = src.join("epris-props.json");

    let obj = values
        .as_object()
        .ok_or_else(|| "values must be an object".to_string())?;

    for k in obj.keys() {
        if !is_ascii_js_identifier(k) {
            return Err(format!("Invalid prop id (must be JS identifier): {}", k));
        }
    }

    let file = serde_json::json!({
        "schemaVersion": 1,
        "values": obj,
    });
    write_json_file(&path, &file)?;
    Ok(file)
}
