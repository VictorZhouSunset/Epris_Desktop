#[cfg(target_os = "windows")]
use crate::environment::EnvironmentManager;
#[cfg(target_os = "windows")]
use crate::state_manager::{StateManager, ToolchainVersion};
#[cfg(target_os = "windows")]
use crate::utils;
#[cfg(target_os = "windows")]
use std::path::{Path, PathBuf};
#[cfg(target_os = "windows")]
use tauri::Manager;
#[cfg(target_os = "windows")]
use crate::install_log;

#[cfg(target_os = "windows")]
const PINNED_NODE_VERSION: &str = "24.13.0";
// Keep pnpm pinned for reproducibility. If you change this, also update docs/workflow.
#[cfg(target_os = "windows")]
const PINNED_PNPM_VERSION: &str = "10.28.2";
#[cfg(target_os = "windows")]
const PINNED_GEMINI_CLI_VERSION: &str = "0.25.0";
// OpenCode upstream has had Windows packaging churn; we pin a version but fall back to latest if unavailable.
#[cfg(target_os = "windows")]
const PINNED_OPENCODE_VERSION: &str = "1.1.48";

// Official Node.js Windows x64 zip.
#[cfg(target_os = "windows")]
const NODE_WIN_X64_ZIP_URL: &str =
    "https://nodejs.org/dist/v24.13.0/node-v24.13.0-win-x64.zip";
// SHA256 for node-v24.13.0-win-x64.zip (from SHASUMS256.txt).
#[cfg(target_os = "windows")]
const NODE_WIN_X64_ZIP_SHA256: &str =
    "ca2742695be8de44027d71b3f53a4bdb36009b95575fe1ae6f7f0b5ce091cb88";

#[cfg(not(target_os = "windows"))]
pub async fn ensure_local_toolchain(
    _window: Option<&tauri::Window>,
    _workspace_path: Option<&std::path::Path>,
    _provider: &str,
    _app_handle: &tauri::AppHandle,
) -> Result<(), String> {
    Err("Local toolchain install is only implemented for Windows right now.".to_string())
}

#[cfg(target_os = "windows")]
pub async fn ensure_local_toolchain(
    window: Option<&tauri::Window>,
    workspace_path: Option<&Path>,
    provider: &str,
    app_handle: &tauri::AppHandle,
) -> Result<(), String> {
    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?;
    let env = EnvironmentManager::new(app_dir.clone());

    emit_log(window, &app_dir, workspace_path, "Toolchain: ensuring local Node.js...")?;
    ensure_node(window, &app_dir, workspace_path, &env).await?;
    emit_log(window, &app_dir, workspace_path, "Toolchain: ensuring local npm shims...")?;
    ensure_npm_shims(&env)?;

    emit_log(window, &app_dir, workspace_path, "Toolchain: ensuring local pnpm...")?;
    ensure_npm_global_package(window, &app_dir, workspace_path, &env, "pnpm", Some(PINNED_PNPM_VERSION)).await?;

    // Provider CLIs (install only the currently selected provider).
    if provider == "gemini" {
        emit_log(window, &app_dir, workspace_path, "Toolchain: ensuring local Gemini CLI...")?;
        if let Err(e) = ensure_npm_global_package(
            window,
            &app_dir,
            workspace_path,
            &env,
            "@google/gemini-cli",
            Some(PINNED_GEMINI_CLI_VERSION),
        )
        .await
        {
            emit_log(
                window,
                &app_dir,
                workspace_path,
                &format!(
                    "Toolchain: Gemini pinned install failed ({}). Falling back to latest...",
                    e
                ),
            )?;
            ensure_npm_global_package(window, &app_dir, workspace_path, &env, "@google/gemini-cli", None).await?;
        }
    } else {
        emit_log(window, &app_dir, workspace_path, "Toolchain: ensuring local OpenCode CLI...")?;
        if let Err(e) =
            ensure_npm_global_package(window, &app_dir, workspace_path, &env, "opencode-ai", Some(PINNED_OPENCODE_VERSION))
                .await
        {
            // Best-effort: OpenCode has seen Windows packaging changes. Fall back to latest.
            emit_log(
                window,
                &app_dir,
                workspace_path,
                &format!(
                    "Toolchain: OpenCode pinned install failed ({}). Falling back to latest...",
                    e
                ),
            )?;
            ensure_npm_global_package(window, &app_dir, workspace_path, &env, "opencode-ai", None).await?;
        }
    }

    // Persist observed tool versions into state.json (best-effort).
    record_toolchain_versions(app_handle, &env, provider);
    emit_log(window, &app_dir, workspace_path, "Toolchain: done.")?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn emit_log(
    window: Option<&tauri::Window>,
    app_data_dir: &Path,
    workspace_path: Option<&Path>,
    line: &str,
) -> Result<(), String> {
    install_log::log_env_install(window, Some(app_data_dir), workspace_path, line);
    Ok(())
}

#[cfg(target_os = "windows")]
async fn ensure_node(
    window: Option<&tauri::Window>,
    app_data_dir: &Path,
    workspace_path: Option<&Path>,
    env: &EnvironmentManager,
) -> Result<(), String> {
    let toolchain_dir = env.toolchain_dir.clone();
    let node_dir = toolchain_dir.join("node");
    let bin_dir = env.get_bin_dir();

    tokio::fs::create_dir_all(&node_dir).await.map_err(|e| e.to_string())?;
    tokio::fs::create_dir_all(&bin_dir).await.map_err(|e| e.to_string())?;

    let node_exe_in_node = node_dir.join("node.exe");
    if node_exe_in_node.exists() {
        if let Ok(v) = run_exe_capture(&node_exe_in_node, &["--version"], &bin_dir) {
            if v.trim().trim_start_matches('v') == PINNED_NODE_VERSION {
                // Ensure node.exe is present in bin (used by other shims).
                let _ = std::fs::copy(&node_exe_in_node, bin_dir.join("node.exe"));
                return Ok(());
            }
        }
    }

    emit_log(
        window,
        app_data_dir,
        workspace_path,
        &format!("Toolchain: downloading Node.js v{}...", PINNED_NODE_VERSION),
    )?;

    let zip_path = toolchain_dir.join(format!("node-v{}-win-x64.zip", PINNED_NODE_VERSION));
    download_file(window, app_data_dir, workspace_path, &zip_path, NODE_WIN_X64_ZIP_URL, "Node.js (zip)").await?;
    verify_sha256(&zip_path, NODE_WIN_X64_ZIP_SHA256)?;

    // Extract into a temp folder, then flatten into node_dir.
    let tmp = toolchain_dir.join(format!("node-extract-{}", uuid::Uuid::new_v4()));
    if tmp.exists() {
        let _ = std::fs::remove_dir_all(&tmp);
    }
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    unzip(&zip_path, &tmp)?;

    // Flatten: Node zip has a single top-level folder "node-vX-win-x64".
    let extracted_root = find_single_child_dir_with(&tmp, "node.exe")
        .unwrap_or(tmp.clone());

    if node_dir.exists() {
        let _ = std::fs::remove_dir_all(&node_dir);
    }
    std::fs::create_dir_all(&node_dir).map_err(|e| e.to_string())?;
    move_dir_contents(&extracted_root, &node_dir)?;
    let _ = std::fs::remove_dir_all(&tmp);

    if !node_dir.join("node.exe").exists() {
        return Err("Node install failed: node.exe missing after extraction".to_string());
    }

    // Copy node.exe into toolchain/bin for shims + PATH usage.
    std::fs::copy(node_dir.join("node.exe"), bin_dir.join("node.exe")).map_err(|e| e.to_string())?;

    // Validate version.
    let v = run_exe_capture(&bin_dir.join("node.exe"), &["--version"], &bin_dir)?;
    if v.trim().trim_start_matches('v') != PINNED_NODE_VERSION {
        return Err(format!(
            "Node version mismatch after install: got {}, expected {}",
            v.trim(),
            PINNED_NODE_VERSION
        ));
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn ensure_npm_shims(env: &EnvironmentManager) -> Result<(), String> {
    let toolchain_dir = env.toolchain_dir.clone();
    let node_dir = toolchain_dir.join("node");
    let bin_dir = env.get_bin_dir();
    std::fs::create_dir_all(&bin_dir).map_err(|e| e.to_string())?;

    let node_exe = bin_dir.join("node.exe");
    if !node_exe.exists() {
        // As a fallback, allow running node directly from node_dir (but prefer copying into bin).
        if node_dir.join("node.exe").exists() {
            let _ = std::fs::copy(node_dir.join("node.exe"), &node_exe);
        }
    }

    write_cmd(
        &bin_dir.join("npm.cmd"),
        &format!(
            "@ECHO OFF\r\nSETLOCAL\r\n\"%~dp0node.exe\" \"%~dp0..\\node\\node_modules\\npm\\bin\\npm-cli.js\" %*\r\n"
        ),
    )?;
    write_cmd(
        &bin_dir.join("npx.cmd"),
        &format!(
            "@ECHO OFF\r\nSETLOCAL\r\n\"%~dp0node.exe\" \"%~dp0..\\node\\node_modules\\npm\\bin\\npx-cli.js\" %*\r\n"
        ),
    )?;
    Ok(())
}

#[cfg(target_os = "windows")]
async fn ensure_npm_global_package(
    window: Option<&tauri::Window>,
    app_data_dir: &Path,
    workspace_path: Option<&Path>,
    env: &EnvironmentManager,
    package: &str,
    version: Option<&str>,
) -> Result<(), String> {
    let bin_dir = env.get_bin_dir();
    std::fs::create_dir_all(&bin_dir).map_err(|e| e.to_string())?;

    // Keep npm cache local to avoid polluting user global cache.
    let npm_cache = env.toolchain_dir.join("npm-cache");
    let _ = std::fs::create_dir_all(&npm_cache);

    let pkg_spec = match version {
        Some(v) => format!("{}@{}", package, v),
        None => package.to_string(),
    };

    emit_log(window, app_data_dir, workspace_path, &format!("Toolchain: npm -g install {}", pkg_spec))?;

    // Ensure npm is callable.
    let npm_cmd = bin_dir.join("npm.cmd");
    if !npm_cmd.exists() {
        return Err("npm shim missing (expected toolchain/bin/npm.cmd)".to_string());
    }

    let prefix = bin_dir.to_string_lossy().to_string();
    let args = ["install", "-g", &pkg_spec, "--prefix", &prefix, "--no-fund", "--no-audit", "--silent"];

    let mut cmd = utils::create_shell_command("npm", &args);
    cmd.env("PATH", prepend_path(&bin_dir)?);
    cmd.env("npm_config_cache", npm_cache.to_string_lossy().to_string());
    cmd.env("npm_config_update_notifier", "false");

    let out = cmd.output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(format!("npm install failed: {}", stderr));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn record_toolchain_versions(app_handle: &tauri::AppHandle, env: &EnvironmentManager, provider: &str) {
    let Ok(app_dir) = app_handle.path().app_local_data_dir() else {
        return;
    };
    let mgr = StateManager::new(app_dir);

    let node_v = run_command_in_toolchain(env, "node", &["--version"])
        .ok()
        .map(|s| s.trim().trim_start_matches('v').to_string());
    let pnpm_v = run_command_in_toolchain(env, "pnpm", &["--version"])
        .ok()
        .map(|s| s.trim().to_string());
    let provider_cmd = if provider == "gemini" { "gemini" } else { "opencode" };
    let provider_v = run_command_in_toolchain(env, provider_cmd, &["--version"])
        .ok()
        .map(|s| s.trim().to_string());

    let _ = mgr.update(|s| {
        if let Some(v) = node_v {
            s.toolchain.node = Some(ToolchainVersion {
                version: v,
                source: "local".into(),
            });
        }
        if let Some(v) = pnpm_v {
            s.toolchain.pnpm = Some(ToolchainVersion {
                version: v,
                source: "local".into(),
            });
        }
        if let Some(v) = provider_v {
            s.toolchain.provider_cli = Some(ToolchainVersion {
                version: v,
                source: "local".into(),
            });
        }
    });
}

#[cfg(target_os = "windows")]
fn prepend_path(bin_dir: &Path) -> Result<String, String> {
    let path_env = std::env::var("PATH").unwrap_or_default();
    Ok(format!("{};{}", bin_dir.to_string_lossy(), path_env))
}

#[cfg(target_os = "windows")]
fn run_command_in_toolchain(
    env: &EnvironmentManager,
    program: &str,
    args: &[&str],
) -> Result<String, String> {
    let bin_dir = env.get_bin_dir();
    let mut cmd = utils::create_shell_command(program, args);
    cmd.env("PATH", prepend_path(&bin_dir)?);
    let out = cmd.output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err("Command failed".to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[cfg(target_os = "windows")]
fn run_exe_capture(exe_path: &Path, args: &[&str], bin_dir: &Path) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    use std::os::windows::process::CommandExt;

    let mut cmd = std::process::Command::new(exe_path);
    cmd.args(args);
    cmd.env("PATH", prepend_path(bin_dir)?);
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let out = cmd.output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err("Command failed".to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[cfg(target_os = "windows")]
async fn download_file(
    window: Option<&tauri::Window>,
    app_data_dir: &Path,
    workspace_path: Option<&Path>,
    path: &Path,
    url: &str,
    label: &str,
) -> Result<(), String> {
    const CONNECT_TIMEOUT_SECS: u64 = 30;
    const REQUEST_TIMEOUT_SECS: u64 = 60 * 60;

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(CONNECT_TIMEOUT_SECS))
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| e.to_string())?;

    let res = client
        .get(url)
        .header("User-Agent", "epris-desktop")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        return Err(format!("Download failed: {} ({})", url, res.status()));
    }

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("part");
    let mut file = tokio::fs::File::create(&tmp).await.map_err(|e| e.to_string())?;
    let mut stream = res.bytes_stream();
    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
    }
    file.flush().await.map_err(|e| e.to_string())?;
    drop(file);
    if path.exists() {
        let _ = tokio::fs::remove_file(path).await;
    }
    tokio::fs::rename(&tmp, path).await.map_err(|e| e.to_string())?;
    emit_log(window, app_data_dir, workspace_path, &format!("Toolchain: downloaded {}", label))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn verify_sha256(path: &Path, expected_hex: &str) -> Result<(), String> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let got = hex::encode(hasher.finalize());
    if got.to_ascii_lowercase() != expected_hex.to_ascii_lowercase() {
        // Best-effort cleanup so the next run can re-download.
        let _ = std::fs::remove_file(path);
        return Err(format!(
            "SHA256 mismatch for {}: got {}, expected {}",
            path.to_string_lossy(),
            got,
            expected_hex
        ));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn unzip(zip_path: &Path, dest_dir: &Path) -> Result<(), String> {
    let f = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let out_path = dest_dir.join(file.name());
        if file.name().ends_with('/') {
            std::fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out = std::fs::File::create(&out_path).map_err(|e| e.to_string())?;
        std::io::copy(&mut file, &mut out).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn find_single_child_dir_with(root: &Path, sentinel_file: &str) -> Option<PathBuf> {
    let entries: Vec<_> = std::fs::read_dir(root).ok()?.flatten().collect();
    if entries.len() != 1 {
        return None;
    }
    let p = entries[0].path();
    if p.is_dir() && p.join(sentinel_file).exists() {
        return Some(p);
    }
    None
}

#[cfg(target_os = "windows")]
fn move_dir_contents(src_dir: &Path, dst_dir: &Path) -> Result<(), String> {
    for entry in std::fs::read_dir(src_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let src = entry.path();
        let name = entry.file_name();
        let dst = dst_dir.join(name);
        if dst.exists() {
            if dst.is_dir() {
                let _ = std::fs::remove_dir_all(&dst);
            } else {
                let _ = std::fs::remove_file(&dst);
            }
        }
        if std::fs::rename(&src, &dst).is_ok() {
            continue;
        }
        if src.is_dir() {
            std::fs::create_dir_all(&dst).map_err(|e| e.to_string())?;
            let mut options = fs_extra::dir::CopyOptions::new();
            options.copy_inside = true;
            fs_extra::dir::copy(&src, &dst, &options).map_err(|e| e.to_string())?;
            let _ = std::fs::remove_dir_all(&src);
        } else {
            std::fs::copy(&src, &dst).map_err(|e| e.to_string())?;
            let _ = std::fs::remove_file(&src);
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn write_cmd(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, content.as_bytes()).map_err(|e| e.to_string())
}
