use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoConfig {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub duration_frames: u32,
    pub duration_seconds: f64,
}

fn parse_export_const_u32(line: &str, name: &str) -> Option<u32> {
    let trimmed = line.trim();
    let prefix = format!("export const {} =", name);
    if !trimmed.starts_with(&prefix) {
        return None;
    }
    let rest = trimmed.strip_prefix(&prefix)?;
    let number = rest
        .trim()
        .trim_end_matches(';')
        .trim()
        .split_whitespace()
        .next()?;
    number.parse::<u32>().ok()
}

fn find_prop_number_u32(text: &str, prop: &str) -> Option<u32> {
    // Very small “parser” for common JSX patterns like: prop={123}
    // We intentionally keep it dependency-free (no regex crate).
    let needle = format!("{}={{", prop);
    let idx = text.find(&needle)?;
    let after = &text[idx + needle.len()..];
    let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse::<u32>().ok()
}

fn find_prop_number_f64(text: &str, prop: &str) -> Option<f64> {
    let needle = format!("{}={{", prop);
    let idx = text.find(&needle)?;
    let after = &text[idx + needle.len()..];
    let num: String = after
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    num.parse::<f64>().ok()
}

fn replace_export_const_u32(lines: &mut [String], name: &str, value: u32) -> bool {
    let needle = format!("export const {}", name);
    for line in lines.iter_mut() {
        let trimmed = line.trim_start();
        if trimmed.starts_with(&needle) {
            let indent_len = line.len().saturating_sub(trimmed.len());
            let indent = " ".repeat(indent_len);
            *line = format!("{}export const {} = {};", indent, name, value);
            return true;
        }
    }
    false
}

fn migrate_legacy_video_config(workspace_path: &str) -> Result<(), String> {
    let ws = Path::new(workspace_path);
    let src_dir = ws.join("src");
    let vc_path = src_dir.join("VideoConfig.ts");

    if vc_path.exists() {
        return Ok(());
    }

    let root_path = src_dir.join("Root.tsx");
    let preview_path = src_dir.join("Preview.tsx");

    let root_text = std::fs::read_to_string(&root_path).unwrap_or_default();
    let preview_text = std::fs::read_to_string(&preview_path).unwrap_or_default();

    // Try to infer current settings from legacy Root/Preview files.
    let root_width = find_prop_number_u32(&root_text, "width");
    let root_height = find_prop_number_u32(&root_text, "height");
    let root_fps = find_prop_number_u32(&root_text, "fps");
    let root_dur = find_prop_number_u32(&root_text, "durationInFrames");

    let preview_width = find_prop_number_u32(&preview_text, "compositionWidth");
    let preview_height = find_prop_number_u32(&preview_text, "compositionHeight");
    let preview_fps = find_prop_number_u32(&preview_text, "fps");
    let preview_dur = find_prop_number_u32(&preview_text, "durationInFrames");

    let width = root_width.or(preview_width).unwrap_or(1920);
    let height = root_height.or(preview_height).unwrap_or(1080);

    // Lock FPS to 30 per Step 13.
    let target_fps: u32 = 30;

    // Estimate duration seconds based on legacy values to preserve length.
    let legacy_seconds = match (root_dur, root_fps) {
        (Some(d), Some(f)) if f > 0 => Some((d as f64) / (f as f64)),
        _ => match (preview_dur, preview_fps) {
            (Some(d), Some(f)) if f > 0 => Some((d as f64) / (f as f64)),
            _ => None,
        },
    }
    .or_else(|| find_prop_number_f64(&root_text, "durationSeconds"))
    .unwrap_or(5.0);

    let mut duration_frames = (legacy_seconds * (target_fps as f64)).round() as i64;
    if duration_frames < 1 {
        duration_frames = 1;
    }
    let duration_frames = duration_frames as u32;

    // Write new VideoConfig.ts
    let content = format!(
        "export const COMPOSITION_ID = \"Main\";\n\
export const VIDEO_WIDTH = {};\n\
export const VIDEO_HEIGHT = {};\n\
export const VIDEO_FPS = {};\n\
\n// Gemini: You MAY modify this value to change the video duration.\n\
export const DURATION_IN_FRAMES = {};\n",
        width, height, target_fps, duration_frames
    );
    std::fs::write(&vc_path, content).map_err(|e| format!("Failed to write VideoConfig.ts: {}", e))?;

    // Rewrite Root.tsx to use VideoConfig.
    if !root_text.is_empty() {
        let new_root = "import { Composition, registerRoot } from 'remotion';\n\
import { Main } from './Composition';\n\
import { COMPOSITION_ID, VIDEO_WIDTH, VIDEO_HEIGHT, VIDEO_FPS, DURATION_IN_FRAMES } from './VideoConfig';\n\
\n\
export const RemotionRoot: React.FC = () => {\n\
\treturn (\n\
\t\t<>\n\
\t\t\t<Composition\n\
\t\t\t\tid={COMPOSITION_ID}\n\
\t\t\t\tcomponent={Main}\n\
\t\t\t\tdurationInFrames={DURATION_IN_FRAMES}\n\
\t\t\t\tfps={VIDEO_FPS}\n\
\t\t\t\twidth={VIDEO_WIDTH}\n\
\t\t\t\theight={VIDEO_HEIGHT}\n\
\t\t\t/>\n\
\t\t</>\n\
\t);\n\
};\n\
\n\
registerRoot(RemotionRoot);\n";
        std::fs::write(&root_path, new_root).map_err(|e| format!("Failed to write Root.tsx: {}", e))?;
    }

    // Rewrite Preview.tsx to use VideoConfig (used by iframe preview).
    if !preview_text.is_empty() {
        let new_preview = "import React from 'react';\n\
import { Player } from '@remotion/player';\n\
import { Main } from './Composition';\n\
import { VIDEO_FPS, DURATION_IN_FRAMES, VIDEO_WIDTH, VIDEO_HEIGHT } from './VideoConfig';\n\
\n\
/**\n\
 * Standalone preview page for embedding in iframe.\n\
 * Renders the Main composition with player controls.\n\
 */\n\
export const Preview: React.FC = () => {\n\
  return (\n\
    <div\n\
      style={{\n\
        width: '100vw',\n\
        height: '100vh',\n\
        background: '#1e293b',\n\
        display: 'flex',\n\
        alignItems: 'center',\n\
        justifyContent: 'center',\n\
      }}\n\
    >\n\
      <Player\n\
        component={Main}\n\
        durationInFrames={DURATION_IN_FRAMES}\n\
        compositionWidth={VIDEO_WIDTH}\n\
        compositionHeight={VIDEO_HEIGHT}\n\
        fps={VIDEO_FPS}\n\
        style={{ width: '100%', height: '100%' }}\n\
        controls\n\
        autoPlay\n\
        loop\n\
      />\n\
    </div>\n\
  );\n\
};\n\
\n\
export default Preview;\n";
        std::fs::write(&preview_path, new_preview).map_err(|e| format!("Failed to write Preview.tsx: {}", e))?;
    }

    Ok(())
}

fn ensure_video_config_wired(workspace_path: &str) -> Result<(), String> {
    let ws = Path::new(workspace_path);
    let src_dir = ws.join("src");
    let vc_path = src_dir.join("VideoConfig.ts");
    if !vc_path.exists() {
        return Ok(());
    }

    // Vite entrypoint: ensure the preview page is actually used.
    // Some legacy projects render <Player> directly in src/index.tsx with hardcoded numbers.
    // That prevents UI updates when VideoConfig.ts changes.
    let index_path = src_dir.join("index.tsx");
    if index_path.exists() {
        let index_text = std::fs::read_to_string(&index_path).unwrap_or_default();
        let looks_like_hardcoded_player = index_text.contains("@remotion/player")
            || index_text.contains("compositionWidth")
            || index_text.contains("compositionHeight")
            || index_text.contains("durationInFrames");

        let already_uses_preview = index_text.contains("from \"./Preview\"")
            || index_text.contains("from './Preview'")
            || index_text.contains("<Preview")
            || index_text.contains("render(<Preview");

        if looks_like_hardcoded_player && !already_uses_preview {
            let new_index = "import React from \"react\";\n\
import ReactDOM from \"react-dom/client\";\n\
import Preview from \"./Preview\";\n\
\n\
ReactDOM.createRoot(document.getElementById(\"root\")!).render(<Preview />);\n";
            std::fs::write(&index_path, new_index)
                .map_err(|e| format!("Failed to write index.tsx: {}", e))?;
        }
    }

    let root_path = src_dir.join("Root.tsx");
    let preview_path = src_dir.join("Preview.tsx");

    if root_path.exists() {
        let root_text = std::fs::read_to_string(&root_path).unwrap_or_default();
        if !root_text.contains("from './VideoConfig'") {
            let new_root = "import { Composition, registerRoot } from 'remotion';\n\
import { Main } from './Composition';\n\
import { COMPOSITION_ID, VIDEO_WIDTH, VIDEO_HEIGHT, VIDEO_FPS, DURATION_IN_FRAMES } from './VideoConfig';\n\
\n\
export const RemotionRoot: React.FC = () => {\n\
\treturn (\n\
\t\t<>\n\
\t\t\t<Composition\n\
\t\t\t\tid={COMPOSITION_ID}\n\
\t\t\t\tcomponent={Main}\n\
\t\t\t\tdurationInFrames={DURATION_IN_FRAMES}\n\
\t\t\t\tfps={VIDEO_FPS}\n\
\t\t\t\twidth={VIDEO_WIDTH}\n\
\t\t\t\theight={VIDEO_HEIGHT}\n\
\t\t\t/>\n\
\t\t</>\n\
\t);\n\
};\n\
\n\
registerRoot(RemotionRoot);\n";
            std::fs::write(&root_path, new_root).map_err(|e| format!("Failed to write Root.tsx: {}", e))?;
        }
    }

    if preview_path.exists() {
        let preview_text = std::fs::read_to_string(&preview_path).unwrap_or_default();
        if !preview_text.contains("from './VideoConfig'") {
            let new_preview = "import React from 'react';\n\
import { Player } from '@remotion/player';\n\
import { Main } from './Composition';\n\
import { VIDEO_FPS, DURATION_IN_FRAMES, VIDEO_WIDTH, VIDEO_HEIGHT } from './VideoConfig';\n\
\n\
/**\n\
 * Standalone preview page for embedding in iframe.\n\
 * Renders the Main composition with player controls.\n\
 */\n\
export const Preview: React.FC = () => {\n\
  return (\n\
    <div\n\
      style={{\n\
        width: '100vw',\n\
        height: '100vh',\n\
        background: '#1e293b',\n\
        display: 'flex',\n\
        alignItems: 'center',\n\
        justifyContent: 'center',\n\
      }}\n\
    >\n\
      <Player\n\
        component={Main}\n\
        durationInFrames={DURATION_IN_FRAMES}\n\
        compositionWidth={VIDEO_WIDTH}\n\
        compositionHeight={VIDEO_HEIGHT}\n\
        fps={VIDEO_FPS}\n\
        style={{ width: '100%', height: '100%' }}\n\
        controls\n\
        autoPlay\n\
        loop\n\
      />\n\
    </div>\n\
  );\n\
};\n\
\n\
export default Preview;\n";
            std::fs::write(&preview_path, new_preview).map_err(|e| format!("Failed to write Preview.tsx: {}", e))?;
        }
    }

    Ok(())
}

fn read_video_config(workspace_path: &str) -> Result<VideoConfig, String> {
    let file_path = Path::new(workspace_path).join("src").join("VideoConfig.ts");
    if !file_path.exists() {
        migrate_legacy_video_config(workspace_path)?;
    }
    ensure_video_config_wired(workspace_path)?;
    let content = std::fs::read_to_string(&file_path).map_err(|e| format!("Failed to read VideoConfig.ts: {}", e))?;
    let mut width = None;
    let mut height = None;
    let mut fps = None;
    let mut duration_frames = None;

    for line in content.lines() {
        if width.is_none() {
            width = parse_export_const_u32(line, "VIDEO_WIDTH");
        }
        if height.is_none() {
            height = parse_export_const_u32(line, "VIDEO_HEIGHT");
        }
        if fps.is_none() {
            fps = parse_export_const_u32(line, "VIDEO_FPS");
        }
        if duration_frames.is_none() {
            duration_frames = parse_export_const_u32(line, "DURATION_IN_FRAMES");
        }
    }

    let width = width.ok_or("VIDEO_WIDTH not found in src/VideoConfig.ts")?;
    let height = height.ok_or("VIDEO_HEIGHT not found in src/VideoConfig.ts")?;
    let fps = fps.ok_or("VIDEO_FPS not found in src/VideoConfig.ts")?;
    let duration_frames = duration_frames.ok_or("DURATION_IN_FRAMES not found in src/VideoConfig.ts")?;

    let duration_seconds = (duration_frames as f64) / (fps as f64);
    Ok(VideoConfig {
        width,
        height,
        fps,
        duration_frames,
        duration_seconds,
    })
}

fn write_video_config(workspace_path: &str, width: u32, height: u32, duration_seconds: f64) -> Result<VideoConfig, String> {
    let file_path = Path::new(workspace_path).join("src").join("VideoConfig.ts");
    if !file_path.exists() {
        migrate_legacy_video_config(workspace_path)?;
    }
    ensure_video_config_wired(workspace_path)?;
    let content = std::fs::read_to_string(&file_path).map_err(|e| format!("Failed to read VideoConfig.ts: {}", e))?;
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

    // For now, lock FPS to 30 per spec.
    let fps: u32 = 30;
    let mut frames = (duration_seconds * (fps as f64)).round() as i64;
    if frames < 1 {
        frames = 1;
    }
    let duration_frames = frames as u32;

    let ok_w = replace_export_const_u32(&mut lines, "VIDEO_WIDTH", width);
    let ok_h = replace_export_const_u32(&mut lines, "VIDEO_HEIGHT", height);
    let ok_fps = replace_export_const_u32(&mut lines, "VIDEO_FPS", fps);
    let ok_dur = replace_export_const_u32(&mut lines, "DURATION_IN_FRAMES", duration_frames);

    if !(ok_w && ok_h && ok_fps && ok_dur) {
        return Err("Failed to update src/VideoConfig.ts (expected exported constants not found)".to_string());
    }

    let new_content = format!("{}\n", lines.join("\n"));
    std::fs::write(&file_path, new_content).map_err(|e| format!("Failed to write VideoConfig.ts: {}", e))?;

    // Ensure Root/Preview are wired after the write as well.
    ensure_video_config_wired(workspace_path)?;

    Ok(VideoConfig {
        width,
        height,
        fps,
        duration_frames,
        duration_seconds: (duration_frames as f64) / (fps as f64),
    })
}

#[tauri::command]
pub fn get_video_config(workspace_path: String) -> Result<VideoConfig, String> {
    read_video_config(&workspace_path)
}

#[tauri::command]
pub fn set_video_config(
    workspace_path: String,
    width: u32,
    height: u32,
    duration_seconds: f64,
) -> Result<VideoConfig, String> {
    if width == 0 || height == 0 {
        return Err("Width/height must be positive".to_string());
    }
    if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
        return Err("Duration must be a positive number".to_string());
    }
    write_video_config(&workspace_path, width, height, duration_seconds)
}
