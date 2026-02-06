use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct EprisObjectMeta {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

fn parse_jsx_string_attr(tag: &str, attr: &str) -> Option<String> {
    let needle = format!("{}=", attr);
    let idx = tag.find(&needle)?;
    let mut rest = &tag[idx + needle.len()..];
    rest = rest.trim_start();
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = &rest[1..];
    let end = rest.find(quote)?;
    Some(rest[..end].to_string())
}

fn scan_epris_groups_in_text(text: &str) -> Vec<EprisObjectMeta> {
    // Very lightweight JSX scan:
    // - finds "<EprisGroup"
    // - parses id/label/kind as string literal attributes (id="...", id='...')
    // - ignores dynamic attrs (id={...})
    let mut out: BTreeMap<String, EprisObjectMeta> = BTreeMap::new();
    let mut search = text;
    while let Some(pos) = search.find("<EprisGroup") {
        let after = &search[pos..];
        let Some(gt) = after.find('>') else {
            break;
        };
        let tag = &after[..gt];
        if let Some(id) = parse_jsx_string_attr(tag, "id") {
            let label = parse_jsx_string_attr(tag, "label");
            let kind = parse_jsx_string_attr(tag, "kind");
            out.entry(id.clone()).or_insert(EprisObjectMeta {
                id,
                label,
                kind,
                tags: None,
            });
        }
        search = &after[gt + 1..];
    }
    out.into_values().collect()
}

#[tauri::command]
pub fn scan_epris_objects(workspace_path: String) -> Result<Vec<EprisObjectMeta>, String> {
    let composition_path = Path::new(&workspace_path)
        .join("src")
        .join("Composition.tsx");
    let bytes = std::fs::read(&composition_path)
        .map_err(|e| format!("Failed to read src/Composition.tsx: {}", e))?;
    let text = String::from_utf8_lossy(&bytes);
    Ok(scan_epris_groups_in_text(&text))
}

