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

fn normalize_epris_props_file(raw: Value) -> Result<(Value, bool), String> {
    let mut obj = match raw {
        Value::Object(map) => map,
        _ => return Err("epris-props.json must be an object".to_string()),
    };

    let mut changed = false;

    match obj.get("schemaVersion") {
        Some(Value::Number(n)) if n.as_i64() == Some(1) => {}
        Some(_) => return Err("epris-props.json schemaVersion must be 1".to_string()),
        None => {
            obj.insert("schemaVersion".to_string(), Value::from(1));
            changed = true;
        }
    }

    match obj.get("values") {
        Some(Value::Object(_)) => {}
        Some(_) => return Err("epris-props.json values must be an object".to_string()),
        None => {
            // Legacy/AI-written shape: treat all non-schema keys as values.
            let keys: Vec<String> = obj.keys().cloned().collect();
            let mut values = serde_json::Map::new();
            for key in keys {
                if key == "schemaVersion" {
                    continue;
                }
                if let Some(v) = obj.remove(&key) {
                    values.insert(key, v);
                }
            }
            obj.insert("values".to_string(), Value::Object(values));
            changed = true;
        }
    }

    Ok((Value::Object(obj), changed))
}

fn normalize_epris_controls_file(raw: Value) -> Result<(Value, bool), String> {
    let mut obj = match raw {
        Value::Object(map) => map,
        _ => return Err("epris-controls.json must be an object".to_string()),
    };

    let mut changed = false;

    match obj.get("schemaVersion") {
        Some(Value::Number(n)) if n.as_i64() == Some(1) => {}
        Some(_) => return Err("epris-controls.json schemaVersion must be 1".to_string()),
        None => {
            obj.insert("schemaVersion".to_string(), Value::from(1));
            changed = true;
        }
    }

    if !obj.contains_key("target") {
        obj.insert(
            "target".to_string(),
            serde_json::json!({ "kind": "composition", "id": "Main" }),
        );
        changed = true;
    }

    let controls = match obj.get_mut("controls") {
        Some(Value::Array(arr)) => arr,
        Some(_) => return Err("epris-controls.json controls must be an array".to_string()),
        None => {
            obj.insert("controls".to_string(), Value::Array(Vec::new()));
            changed = true;
            obj.get_mut("controls")
                .and_then(Value::as_array_mut)
                .ok_or_else(|| "epris-controls.json controls must be an array".to_string())?
        }
    };

    for (idx, control) in controls.iter_mut().enumerate() {
        let Some(cobj) = control.as_object_mut() else {
            continue;
        };
        let control_type = cobj
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if control_type == "boolean" && cobj.get("ui").and_then(Value::as_str) == Some("checkbox") {
            cobj.insert("ui".to_string(), Value::String("toggle".to_string()));
            changed = true;
        }

        let Some(ui) = cobj.get("ui").and_then(Value::as_str) else {
            continue;
        };
        let ui_ok = match control_type.as_str() {
            "number" => matches!(ui, "slider" | "input"),
            "color" => ui == "color",
            "select" => ui == "select",
            "boolean" => matches!(ui, "toggle" | "checkbox"),
            "text" => matches!(ui, "text" | "textarea"),
            _ => true,
        };
        if !ui_ok {
            return Err(format!(
                "epris-controls.json controls[{}].ui '{}' invalid for type '{}'",
                idx, ui, control_type
            ));
        }
    }

    Ok((Value::Object(obj), changed))
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
    let raw = read_json_file(&path)?;
    let (normalized, changed) = normalize_epris_controls_file(raw)?;
    if changed {
        let _ = write_json_file(&path, &normalized);
    }
    Ok(normalized)
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
    let raw = read_json_file(&path)?;
    let (normalized, changed) = normalize_epris_props_file(raw)?;
    if changed {
        // Self-heal malformed/legacy files so frontend schema validation stays stable.
        let _ = write_json_file(&path, &normalized);
    }
    Ok(normalized)
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
