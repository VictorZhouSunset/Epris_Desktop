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

fn read_video_config(workspace_path: &str) -> Result<VideoConfig, String> {
    let file_path = Path::new(workspace_path).join("src").join("VideoConfig.ts");
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

