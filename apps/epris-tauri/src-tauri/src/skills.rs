use crate::environment::EnvironmentManager;
use crate::state_manager::{RemotionSkillState, StateManager};
use crate::utils;
use chrono::Utc;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tauri::Emitter;
use uuid::Uuid;
use walkdir::WalkDir;
use zip::ZipArchive;

pub struct SkillsManager;

impl SkillsManager {
    pub async fn install_remotion_skills(
        window: Option<&tauri::Window>,
        workspace_path: &str,
        _provider: &str,
        env_manager: &EnvironmentManager,
    ) -> Result<(), String> {
        let ws_path = Path::new(workspace_path);

        // If skills already exist in both projections, keep this call cheap.
        if workspace_has_any_skill(ws_path.join(".gemini").join("skills"))
            && workspace_has_any_skill(ws_path.join(".opencode").join("skills"))
        {
            return Ok(());
        }

        let toolchain_dir = env_manager.toolchain_dir.clone();
        let app_data_dir = toolchain_dir
            .parent()
            .ok_or("Invalid toolchain_dir (no parent)")?
            .to_path_buf();
        let state_mgr = StateManager::new(app_data_dir);

        let source = "github:remotion-dev/skills".to_string();
        let cache_root = toolchain_dir.join("remotion-skills-cache");

        if let Some(w) = window {
            let _ = w.emit(
                "env_install_log",
                "Preparing Remotion skills cache (pinned commit)...".to_string(),
            );
        }

        let desired_commit = match state_mgr.read().toolchain.remotion_skills {
            Some(s) if !s.commit.is_empty() => s.commit,
            _ => {
                if let Some(w) = window {
                    let _ = w.emit(
                        "env_install_log",
                        "Resolving latest Remotion skills commit...".to_string(),
                    );
                }
                resolve_latest_commit(&source).await?
            }
        };

        ensure_cache_for_commit(window, &cache_root, &source, &desired_commit).await?;

        // Persist pin (newest-on-first-install, stable thereafter).
        let _ = state_mgr.update(|s| {
            s.toolchain.remotion_skills = Some(RemotionSkillState {
                source: source.clone(),
                commit: desired_commit.clone(),
                installed_at: Utc::now().to_rfc3339(),
            });
        })?;

        // Project to workspace (both providers, regardless of selected provider).
        if let Some(w) = window {
            let _ = w.emit(
                "env_install_log",
                "Syncing skills into workspace (.gemini/skills + .opencode/skills)...".to_string(),
            );
        }

        let cache_dir = cache_root.join(&desired_commit);
        sync_skills_into_workspace(&cache_dir, ws_path).map_err(|e| format!("Skill sync failed: {}", e))?;

        if let Some(w) = window {
            let _ = w.emit("env_install_log", "Skills installed successfully!".to_string());
        }

        Ok(())
    }
}

fn workspace_has_any_skill(skills_dir: PathBuf) -> bool {
    if !skills_dir.exists() {
        return false;
    }
    WalkDir::new(skills_dir)
        .max_depth(3)
        .into_iter()
        .filter_map(Result::ok)
        .any(|e| e.file_type().is_file() && e.file_name().to_string_lossy().eq_ignore_ascii_case("SKILL.md"))
}

async fn resolve_latest_commit(source: &str) -> Result<String, String> {
    let remote = "https://github.com/remotion-dev/skills.git";

    // Prefer git if available: git ls-remote <remote> HEAD
    if git_available().await {
        let output = utils::create_async_shell_command(
            "git",
            &["ls-remote", remote, "HEAD"],
        )
        .output()
        .await
        .map_err(|e| format!("git ls-remote failed: {}", e))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = stdout.lines().next() {
                let sha = line.split_whitespace().next().unwrap_or("").trim().to_string();
                if sha.len() >= 7 {
                    return Ok(sha);
                }
            }
        }
    }

    // Archive fallback: GitHub API (no git required)
    if source != "github:remotion-dev/skills" {
        return Err(format!("Unknown skills source: {}", source));
    }

    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("epris-desktop"));
    let client = reqwest::Client::new();
    let res = client
        .get("https://api.github.com/repos/remotion-dev/skills/commits?per_page=1")
        .headers(headers)
        .send()
        .await
        .map_err(|e| format!("GitHub API request failed: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("GitHub API returned {}", res.status()));
    }

    let body: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    let sha = body
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("sha"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if sha.len() < 7 {
        return Err("Failed to parse latest commit SHA from GitHub API".to_string());
    }
    Ok(sha)
}

async fn git_available() -> bool {
    let out = utils::create_async_shell_command("git", &["--version"]).output().await;
    match out {
        Ok(o) => o.status.success(),
        Err(_) => false,
    }
}

async fn ensure_cache_for_commit(
    window: Option<&tauri::Window>,
    cache_root: &Path,
    source: &str,
    commit: &str,
) -> Result<(), String> {
    let cache_dir = cache_root.join(commit);
    if cache_dir.exists() {
        return Ok(());
    }

    std::fs::create_dir_all(cache_root).map_err(|e| e.to_string())?;

    // Prefer git-based if available; fall back to archive-based.
    if git_available().await {
        if let Some(w) = window {
            let _ = w.emit("env_install_log", "Fetching skills via git...".to_string());
        }
        if fetch_via_git(cache_root, commit).await.is_ok() {
            return Ok(());
        }
    }

    if let Some(w) = window {
        let _ = w.emit(
            "env_install_log",
            "Fetching skills via archive download (no git)...".to_string(),
        );
    }
    if source != "github:remotion-dev/skills" {
        return Err(format!("Unknown skills source: {}", source));
    }
    fetch_via_zip_archive(cache_root, commit).await
}

async fn fetch_via_git(cache_root: &Path, commit: &str) -> Result<(), String> {
    let remote = "https://github.com/remotion-dev/skills.git";
    let tmp_dir = cache_root.join(format!(".tmp-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;

    let init = utils::create_async_shell_command("git", &["init"])
        .current_dir(&tmp_dir)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !init.status.success() {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Err("git init failed".to_string());
    }

    let remote_add = utils::create_async_shell_command("git", &["remote", "add", "origin", remote])
        .current_dir(&tmp_dir)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !remote_add.status.success() {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Err("git remote add failed".to_string());
    }

    let fetch = utils::create_async_shell_command("git", &["fetch", "--depth", "1", "origin", commit])
        .current_dir(&tmp_dir)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !fetch.status.success() {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Err("git fetch failed".to_string());
    }

    let checkout = utils::create_async_shell_command("git", &["checkout", "FETCH_HEAD"])
        .current_dir(&tmp_dir)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !checkout.status.success() {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Err("git checkout failed".to_string());
    }

    finalize_cache_dir(cache_root, &tmp_dir, commit)?;
    let _ = std::fs::remove_dir_all(&tmp_dir);
    Ok(())
}

async fn fetch_via_zip_archive(cache_root: &Path, commit: &str) -> Result<(), String> {
    let tmp_dir = cache_root.join(format!(".tmp-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let zip_path = tmp_dir.join("skills.zip");

    let url = format!("https://codeload.github.com/remotion-dev/skills/zip/{}", commit);
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("epris-desktop"));
    let client = reqwest::Client::new();
    let res = client
        .get(url)
        .headers(headers)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    if !res.status().is_success() {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Err(format!("Download returned {}", res.status()));
    }

    let bytes = res.bytes().await.map_err(|e| e.to_string())?;
    let mut f = File::create(&zip_path).map_err(|e| e.to_string())?;
    f.write_all(&bytes).map_err(|e| e.to_string())?;

    let extract_dir = tmp_dir.join("extract");
    std::fs::create_dir_all(&extract_dir).map_err(|e| e.to_string())?;
    if let Err(e) = unzip(&zip_path, &extract_dir) {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Err(e);
    }

    // The archive typically extracts into a single top-level folder (e.g. skills-<sha>/).
    let root = match find_single_directory_child(&extract_dir) {
        Ok(r) => r,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return Err(e);
        }
    };
    if let Err(e) = finalize_cache_dir(cache_root, &root, commit) {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Err(e);
    }

    // Cleanup leftover temp.
    let _ = std::fs::remove_dir_all(&tmp_dir);
    Ok(())
}

fn unzip(zip_path: &Path, dest: &Path) -> Result<(), String> {
    let file = File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;

    for i in 0..archive.len() {
        let mut zf = archive.by_index(i).map_err(|e| e.to_string())?;
        let Some(name) = zf.enclosed_name().map(|p| p.to_owned()) else {
            continue;
        };

        let outpath = dest.join(name);
        if zf.is_dir() {
            std::fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
            continue;
        }

        if let Some(parent) = outpath.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let mut outfile = File::create(&outpath).map_err(|e| e.to_string())?;
        let mut buf = Vec::new();
        zf.read_to_end(&mut buf).map_err(|e| e.to_string())?;
        outfile.write_all(&buf).map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn find_single_directory_child(parent: &Path) -> Result<PathBuf, String> {
    let mut dirs = Vec::new();
    for entry in std::fs::read_dir(parent).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            dirs.push(entry.path());
        }
    }
    if dirs.len() != 1 {
        return Err(format!(
            "Expected a single top-level directory in extracted archive, found {}",
            dirs.len()
        ));
    }
    Ok(dirs.remove(0))
}

fn finalize_cache_dir(cache_root: &Path, prepared_dir: &Path, commit: &str) -> Result<(), String> {
    let final_dir = cache_root.join(commit);
    if final_dir.exists() {
        return Ok(());
    }

    // Try atomic rename; fall back to copy+remove if needed.
    if std::fs::rename(prepared_dir, &final_dir).is_ok() {
        return Ok(());
    }

    let mut copy_options = fs_extra::dir::CopyOptions::new();
    copy_options.copy_inside = true;
    fs_extra::dir::copy(prepared_dir, &final_dir, &copy_options).map_err(|e| e.to_string())?;
    Ok(())
}

fn sync_skills_into_workspace(cache_dir: &Path, ws_path: &Path) -> std::io::Result<()> {
    let skills_root = cache_dir.join("skills");
    let skill_dirs: Vec<PathBuf> = if skills_root.exists() {
        std::fs::read_dir(&skills_root)?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.is_dir() && p.join("SKILL.md").exists())
            .collect()
    } else if cache_dir.join("SKILL.md").exists() {
        vec![cache_dir.to_path_buf()]
    } else {
        Vec::new()
    };

    if skill_dirs.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No skills found in cache",
        ));
    }

    let opencode_base = ws_path.join(".opencode").join("skills");
    let gemini_base = ws_path.join(".gemini").join("skills");
    std::fs::create_dir_all(&opencode_base)?;
    std::fs::create_dir_all(&gemini_base)?;

    for skill_dir in skill_dirs {
        let name = skill_dir
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "skill".to_string());

        let opencode_target = opencode_base.join(&name);
        let gemini_target = gemini_base.join(&name);

        if opencode_target.exists() {
            let _ = std::fs::remove_dir_all(&opencode_target);
        }
        if gemini_target.exists() {
            let _ = std::fs::remove_dir_all(&gemini_target);
        }

        copy_dir_robust(&skill_dir, &opencode_target)?;
        copy_dir_robust(&skill_dir, &gemini_target)?;
    }

    Ok(())
}

// Robust deep copy that handles directories recursively and skips symlinks for safety
fn copy_dir_robust(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    if src.is_symlink() {
        // Skip symlinks; we want the actual content
        return Ok(());
    }
    
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let path = entry.path();
        
        if metadata.is_dir() {
            copy_dir_robust(&path, &dst.join(entry.file_name()))?;
        } else if metadata.is_file() {
            std::fs::copy(&path, dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}
