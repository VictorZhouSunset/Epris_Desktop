use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager};
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use futures_util::StreamExt;
use tokio::sync::mpsc;

static STT_CANCEL_REQUESTED: AtomicBool = AtomicBool::new(false);
static STT_CURRENT_PID: AtomicU32 = AtomicU32::new(0);

fn kill_pid_tree(pid: u32) {
    if pid == 0 {
        return;
    }
    println!("[Epris] Killing STT process tree for PID: {}", pid);

    #[cfg(target_os = "windows")]
    {
        #[cfg(target_os = "windows")]
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .creation_flags(0x08000000)
            .status();
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status();
    }
}

fn toolchain_dir(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?;
    Ok(app_dir.join("toolchain").join("stt").join("whispercpp"))
}

fn whisper_bin_path(root: &Path) -> PathBuf {
    // Deprecated in upstream whisper.cpp; keep as a fallback for older installs.
    if cfg!(target_os = "windows") {
        root.join("whisper-cli.exe")
    } else {
        root.join("whisper-cli")
    }
}

fn models_dir(root: &Path) -> PathBuf {
    root.join("models")
}

fn resolve_whisper_bin(root: &Path) -> Option<PathBuf> {
    // Prefer the non-deprecated binary name used by recent whisper.cpp releases.
    // (We still keep fallbacks for older installs.)
    #[cfg(target_os = "windows")]
    let candidates = ["whisper-whisper-cli.exe", "whisper-cli.exe", "main.exe"];
    #[cfg(not(target_os = "windows"))]
    let candidates = ["whisper-whisper-cli", "whisper-cli", "main"];

    for name in candidates {
        let p = root.join(name);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn copy_whisper_bundle_windows(bin_src: &Path, root: &Path) -> Result<usize, String> {
    let Some(bundle_dir) = bin_src.parent() else {
        return Err("Invalid whisper.cpp binary path (no parent dir)".to_string());
    };
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;

    let mut copied = 0usize;
    for entry in std::fs::read_dir(bundle_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let ft = entry.file_type().map_err(|e| e.to_string())?;
        if !ft.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext != "exe" && ext != "dll" {
            continue;
        }
        let name = entry.file_name();
        let dst = root.join(name);
        std::fs::copy(&path, &dst).map_err(|e| e.to_string())?;
        copied += 1;
    }

    Ok(copied)
}

fn selected_model_path(app_handle: &tauri::AppHandle, root: &Path) -> Result<PathBuf, String> {
    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?;
    let mgr = crate::state_manager::StateManager::new(app_dir);
    let state = mgr.read();
    if let Some(fname) = state.stt_whisper_model {
        return Ok(models_dir(root).join(fname));
    }
    // Fallback: first *.bin in models dir.
    let md = models_dir(root);
    if let Ok(entries) = std::fs::read_dir(&md) {
        for e in entries.flatten() {
            if e.path().extension().and_then(|x| x.to_str()) == Some("bin") {
                return Ok(e.path());
            }
        }
    }
    Err("No whisper.cpp model installed".to_string())
}

fn selected_task(app_handle: &tauri::AppHandle) -> Result<String, String> {
    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?;
    let mgr = crate::state_manager::StateManager::new(app_dir);
    let state = mgr.read();
    Ok(state
        .stt_whisper_task
        .unwrap_or_else(|| "transcribe".to_string()))
}

fn infer_language_arg(model_path: &Path) -> Option<&'static str> {
    let name = model_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    // `.en` models are English-only; for multilingual models let whisper auto-detect.
    if name.contains(".en.") || name.ends_with(".en.bin") {
        Some("en")
    } else {
        None
    }
}

fn infer_language_arg_for_cli(model_path: &Path) -> &'static str {
    // Many whisper.cpp CLI builds default `-l` to `en` when not specified. Explicitly pass
    // `auto` for multilingual models so Chinese doesn't get forced into English.
    infer_language_arg(model_path).unwrap_or("auto")
}

fn read_model_header(path: &Path) -> Result<([u8; 4], u64), String> {
    use std::io::Read;
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hdr = [0u8; 4];
    f.read_exact(&mut hdr).map_err(|e| e.to_string())?;
    Ok((hdr, meta.len()))
}

fn is_known_whisper_model_magic(hdr: &[u8; 4]) -> bool {
    // whisper.cpp ggml models begin with u32 magic 0x67676d6c ("ggml" as an integer), which
    // appears in bytes as "lmgg". GGUF models begin with "GGUF".
    hdr == b"lmgg" || hdr == b"GGUF" || hdr == b"ggml"
}

fn validate_model_file(path: &Path) -> Result<(), String> {
    // Fail fast on obviously broken/corrupted installs.
    const MIN_BYTES: u64 = 1024 * 1024; // 1MB
    let (hdr, size) = read_model_header(path)?;
    if size < MIN_BYTES {
        return Err(format!(
            "Model file is too small ({} bytes): {}",
            size,
            path.display()
        ));
    }
    if !is_known_whisper_model_magic(&hdr) {
        let magic = String::from_utf8_lossy(&hdr).to_string();
        return Err(format!(
            "Model file has unexpected header magic {:?}: {}",
            magic,
            path.display()
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Default)]
struct WhisperCliCaps {
    supports_task: bool,
    supports_translate: bool,
    supports_tr_short: bool,
}

async fn detect_whisper_cli_caps(bin: &Path) -> Result<WhisperCliCaps, String> {
    async fn run_help(bin: &Path, arg: &str) -> Result<String, String> {
        #[cfg(target_os = "windows")]
        let mut cmd = tokio::process::Command::new(bin);
        #[cfg(not(target_os = "windows"))]
        let mut cmd = tokio::process::Command::new(bin);

        #[cfg(target_os = "windows")]
        {
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        let out = cmd
            .arg(arg)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?
            .wait_with_output()
            .await
            .map_err(|e| e.to_string())?;

        let mut s = String::new();
        s.push_str(&String::from_utf8_lossy(&out.stdout));
        s.push_str(&String::from_utf8_lossy(&out.stderr));
        Ok(s)
    }

    // Some whisper.cpp CLIs print help for either --help or -h. Try both.
    let help = match tokio::time::timeout(Duration::from_secs(2), run_help(bin, "--help")).await {
        Ok(Ok(s)) if !s.trim().is_empty() => s,
        _ => match tokio::time::timeout(Duration::from_secs(2), run_help(bin, "-h")).await {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => return Err(e),
            Err(_) => return Err("Timed out running whisper.cpp --help".to_string()),
        },
    };

    let lower = help.to_ascii_lowercase();
    Ok(WhisperCliCaps {
        supports_task: lower.contains("--task"),
        supports_translate: lower.contains("--translate"),
        supports_tr_short: lower.contains("-tr"),
    })
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SttStatus {
    pub installed: bool,
    pub binary_ok: bool,
    pub model_ok: bool,
    pub model: Option<String>,
    pub task: Option<String>,
}

#[tauri::command]
pub fn get_stt_status(app_handle: tauri::AppHandle) -> Result<SttStatus, String> {
    let root = toolchain_dir(&app_handle)?;
    let binary_ok = resolve_whisper_bin(&root).is_some();

    let model_path = selected_model_path(&app_handle, &root).ok();
    let model_ok = model_path
        .as_ref()
        .is_some_and(|p| p.exists() && validate_model_file(p).is_ok());

    Ok(SttStatus {
        installed: binary_ok && model_ok,
        binary_ok,
        model_ok,
        model: model_path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|s| s.to_string()),
        task: selected_task(&app_handle).ok(),
    })
}

#[tauri::command]
pub fn set_stt_task(app_handle: tauri::AppHandle, task: String) -> Result<SttStatus, String> {
    let task_norm = task.to_ascii_lowercase();
    let task_norm = match task_norm.as_str() {
        "transcribe" => "transcribe".to_string(),
        "translate" => "translate".to_string(),
        other => return Err(format!("Unsupported task: {} (use transcribe or translate)", other)),
    };

    let app_dir = app_handle.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let mgr = crate::state_manager::StateManager::new(app_dir);
    let _ = mgr.update(|s| {
        s.stt_whisper_task = Some(task_norm.clone());
    })?;

    get_stt_status(app_handle)
}

async fn download_to_with_progress(
    window: Option<&tauri::Window>,
    path: &Path,
    url: &str,
    label: &str,
) -> Result<(), String> {
    const CONNECT_TIMEOUT_SECS: u64 = 30;
    // Default reqwest has no request timeout; set one so installs can't hang forever.
    const REQUEST_TIMEOUT_SECS: u64 = 60 * 60; // 1 hour
    const EMIT_EVERY_MILLIS: u64 = 500;

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

    let total = res.content_length();
    if let Some(w) = window {
        let _ = w.emit(
            "stt_install_progress",
            serde_json::json!({
                "label": label,
                "downloaded_bytes": 0u64,
                "total_bytes": total,
                "percent": 0.0,
            }),
        );
    }

    let part_path = path.with_extension(format!(
        "{}part",
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| format!("{}.", e))
            .unwrap_or_default()
    ));
    // Write to a temp file first so partial downloads don't get treated as "installed".
    let mut file = tokio::fs::File::create(&part_path)
        .await
        .map_err(|e| e.to_string())?;
    let mut downloaded: u64 = 0;
    let mut last_emit = std::time::Instant::now();

    let mut stream = res.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(c) => c,
            Err(e) => {
                let _ = tokio::fs::remove_file(&part_path).await;
                return Err(e.to_string());
            }
        };
        if let Err(e) = file.write_all(&chunk).await {
            let _ = tokio::fs::remove_file(&part_path).await;
            return Err(e.to_string());
        }
        downloaded = downloaded.saturating_add(chunk.len() as u64);

        if last_emit.elapsed() >= std::time::Duration::from_millis(EMIT_EVERY_MILLIS) {
            if let Some(w) = window {
                let percent = match total {
                    Some(t) if t > 0 => (downloaded as f64 / t as f64) * 100.0,
                    _ => 0.0,
                };
                let _ = w.emit(
                    "stt_install_progress",
                    serde_json::json!({
                        "label": label,
                        "downloaded_bytes": downloaded,
                        "total_bytes": total,
                        "percent": percent,
                    }),
                );
            }
            last_emit = std::time::Instant::now();
        }
    }

    file.flush().await.map_err(|e| e.to_string())?;
    drop(file);

    if let Some(t) = total {
        if downloaded != t {
            let _ = tokio::fs::remove_file(&part_path).await;
            return Err(format!(
                "Download incomplete for {} ({}): got {} bytes, expected {} bytes. Please retry.",
                label, url, downloaded, t
            ));
        }
    }

    // Atomically move into place (best-effort on Windows).
    if path.exists() {
        let _ = tokio::fs::remove_file(path).await;
    }
    tokio::fs::rename(&part_path, path)
        .await
        .map_err(|e| {
            // Cleanup temp file if rename fails.
            let _ = std::fs::remove_file(&part_path);
            e.to_string()
        })?;
    if let Some(w) = window {
        let percent = match total {
            Some(t) if t > 0 => 100.0,
            _ => 0.0,
        };
        let _ = w.emit(
            "stt_install_progress",
            serde_json::json!({
                "label": label,
                "downloaded_bytes": downloaded,
                "total_bytes": total,
                "percent": percent,
                "done": true,
            }),
        );
    }
    Ok(())
}

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

#[derive(Debug, Deserialize)]
struct GithubRelease {
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

fn pick_windows_zip_asset(release: &GithubRelease) -> Option<&GithubAsset> {
    let has_win32 = release
        .assets
        .iter()
        .any(|a| a.name.to_lowercase().contains("win32"));

    let mut candidates: Vec<(&GithubAsset, i32)> = release
        .assets
        .iter()
        .filter_map(|a| {
            let n = a.name.to_lowercase();
            if !n.ends_with(".zip") {
                return None;
            }
            // Exclude obviously-non-Windows zips.
            if n.contains("xcframework") || n.ends_with(".jar.zip") || n.contains("android") {
                return None;
            }
            // Exclude 32-bit Windows.
            if n.contains("win32") {
                return None;
            }
            // Common x64 markers (whisper.cpp has used e.g. `-x64.zip`).
            let is_x64 = n.contains("x64") || n.contains("win64") || n.contains("amd64") || n.contains("x86_64");
            if !is_x64 {
                return None;
            }

            // Score candidates. Prefer the plain CPU `whisper-bin-x64.zip` if present.
            let mut score = 0;
            if n.contains("win") || n.contains("windows") {
                score += 50;
            }
            // Some releases use `whisper-bin-x64.zip` without an explicit `win` marker,
            // but also include a `*-Win32.zip` sibling, which implies the x64 zip is Windows.
            if has_win32 && !(n.contains("win") || n.contains("windows")) {
                score += 40;
            }
            if n.contains("whisper") {
                score += 10;
            }
            if n.contains("bin") {
                score += 20;
            }
            if n.contains("whisper-bin-x64") {
                score += 30;
            }
            if n.contains("blas") {
                score -= 5;
            }
            if n.contains("cublas") || n.contains("cuda") {
                score -= 10;
            }

            Some((a, score))
        })
        .collect();

    candidates.sort_by(|(a1, s1), (a2, s2)| s2.cmp(s1).then_with(|| a1.name.len().cmp(&a2.name.len())));
    candidates.first().map(|(a, _)| *a)
}

fn model_url_and_filename(model: &str) -> Result<(String, String), String> {
    match model {
        "tiny" => Ok((
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin".into(),
            "ggml-tiny.bin".into(),
        )),
        "tiny.en" => Ok((
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin".into(),
            "ggml-tiny.en.bin".into(),
        )),
        "small" => Ok((
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin".into(),
            "ggml-small.bin".into(),
        )),
        _ => Err("Unsupported model (use tiny, tiny.en, or small)".to_string()),
    }
}

#[tauri::command]
pub fn set_stt_model(app_handle: tauri::AppHandle, model: String) -> Result<SttStatus, String> {
    let root = toolchain_dir(&app_handle)?;
    std::fs::create_dir_all(models_dir(&root)).map_err(|e| e.to_string())?;

    let fname = match model.as_str() {
        "tiny" | "tiny.en" | "small" => model_url_and_filename(&model)?.1,
        other => other.to_string(),
    };

    let model_path = models_dir(&root).join(&fname);
    if !model_path.exists() {
        return Err(format!("Model not found: {}. Install it first.", fname));
    }
    validate_model_file(&model_path)?;

    let app_dir = app_handle.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let mgr = crate::state_manager::StateManager::new(app_dir);
    let _ = mgr.update(|s| {
        s.stt_whisper_model = Some(fname.clone());
    })?;

    get_stt_status(app_handle)
}

#[tauri::command]
pub async fn install_whispercpp(
    window: tauri::Window,
    app_handle: tauri::AppHandle,
    model: String,
) -> Result<SttStatus, String> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = window.emit("stt_install_log", "Only Windows is supported for now.".to_string());
        return Err("Only Windows is supported for now".to_string());
    }

    let root = toolchain_dir(&app_handle)?;
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(models_dir(&root)).map_err(|e| e.to_string())?;

    let _ = window.emit("stt_install_log", "Fetching latest whisper.cpp release...".to_string());
    let release: GithubRelease = reqwest::Client::new()
        .get("https://api.github.com/repos/ggerganov/whisper.cpp/releases/latest")
        .header("User-Agent", "epris-desktop")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let Some(asset) = pick_windows_zip_asset(&release) else {
        let mut names: Vec<String> = release.assets.iter().map(|a| a.name.clone()).collect();
        names.sort();
        for n in names.iter().take(25) {
            let _ = window.emit("stt_install_log", format!("Release asset: {}", n));
        }
        return Err("Could not find a Windows x64 whisper.cpp zip in the latest release assets (see stt_install_log for asset list).".to_string());
    };

    let tmp_dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let zip_path = tmp_dir.path().join(&asset.name);
    let _ = window.emit("stt_install_log", format!("Downloading whisper.cpp: {}", asset.name));
    download_to_with_progress(
        Some(&window),
        &zip_path,
        &asset.browser_download_url,
        "Downloading whisper.cpp",
    )
    .await?;

    let extract_dir = tmp_dir.path().join("extract");
    std::fs::create_dir_all(&extract_dir).map_err(|e| e.to_string())?;
    let _ = window.emit("stt_install_log", "Extracting whisper.cpp...".to_string());
    tauri::async_runtime::spawn_blocking({
        let zip_path = zip_path.clone();
        let extract_dir = extract_dir.clone();
        move || unzip(&zip_path, &extract_dir)
    })
    .await
    .map_err(|e| e.to_string())??;

    // Find a plausible CLI executable.
    let mut found_preferred: Option<PathBuf> = None;
    let mut found_deprecated: Option<PathBuf> = None;
    let mut found_main: Option<PathBuf> = None;
    for entry in walkdir::WalkDir::new(&extract_dir).max_depth(5) {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if name == "whisper-whisper-cli.exe" {
            found_preferred = Some(entry.path().to_path_buf());
            break;
        }
        if name == "whisper-cli.exe" {
            found_deprecated = Some(entry.path().to_path_buf());
            continue;
        }
        if name == "main.exe" {
            found_main = Some(entry.path().to_path_buf());
        }
    }

    let bin_src = found_preferred.or(found_deprecated).or(found_main).ok_or_else(|| {
        "Downloaded zip did not contain a recognizable whisper.cpp CLI (expected whisper-whisper-cli.exe / whisper-cli.exe / main.exe).".to_string()
    })?;

    // Copy the whole "bundle" (exe + dlls in the same directory) into our toolchain dir.
    // Copying only the exe can fail at runtime with STATUS_DLL_NOT_FOUND (0xC0000135).
    let copied = copy_whisper_bundle_windows(&bin_src, &root)?;
    let _ = window.emit(
        "stt_install_log",
        format!("Installed whisper.cpp bundle ({} files) into {}", copied, root.display()),
    );

    // Cleanup: if we just installed the preferred binary, remove an older deprecated one to avoid
    // accidentally running it later.
    #[cfg(target_os = "windows")]
    {
        if bin_src
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.eq_ignore_ascii_case("whisper-whisper-cli.exe"))
        {
            let deprecated = whisper_bin_path(&root);
            if deprecated.exists() {
                let _ = std::fs::remove_file(deprecated);
            }
        }
    };

    let (model_url, model_filename) = model_url_and_filename(&model)?;
    let model_dst = models_dir(&root).join(&model_filename);
    let _ = window.emit("stt_install_log", format!("Downloading model: {}", model_filename));
    download_to_with_progress(Some(&window), &model_dst, &model_url, "Downloading model").await?;
    if let Err(e) = validate_model_file(&model_dst) {
        let _ = window.emit("stt_install_log", format!("Model validation failed: {}", e));
        let _ = std::fs::remove_file(&model_dst);
        return Err(format!("Downloaded model is invalid/corrupted. {}", e));
    }

    // Persist selected model to state.json
    let app_dir = app_handle.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let mgr = crate::state_manager::StateManager::new(app_dir);
    let _ = mgr.update(|s| {
        s.stt_whisper_model = Some(model_filename.clone());
    })?;

    get_stt_status(app_handle)
}

#[tauri::command]
pub fn cancel_stt() -> Result<(), String> {
    STT_CANCEL_REQUESTED.store(true, Ordering::SeqCst);
    let pid = STT_CURRENT_PID.swap(0, Ordering::SeqCst);
    if pid != 0 {
        kill_pid_tree(pid);
    }
    Ok(())
}

#[tauri::command]
pub async fn transcribe_whispercpp(
    window: tauri::Window,
    app_handle: tauri::AppHandle,
    workspace_path: String,
    wav_base64: String,
) -> Result<String, String> {
    STT_CANCEL_REQUESTED.store(false, Ordering::SeqCst);
    STT_CURRENT_PID.store(0, Ordering::SeqCst);

    let root = toolchain_dir(&app_handle)?;
    let bin = resolve_whisper_bin(&root).ok_or("whisper.cpp is not installed".to_string())?;
    if !bin.exists() {
        return Err("whisper.cpp is not installed".to_string());
    }
    let model_path = selected_model_path(&app_handle, &root)?;
    if !model_path.exists() {
        return Err("whisper.cpp model not found".to_string());
    }
    let lang = infer_language_arg_for_cli(&model_path);

    // Validate model before invoking whisper.cpp to provide actionable errors.
    let (hdr, size) = read_model_header(&model_path)?;
    let magic = String::from_utf8_lossy(&hdr).to_string();
    let _ = window.emit(
        "stt_log",
        format!(
            "Using model: {} ({} MB, magic {:?})",
            model_path.display(),
            format!("{:.1}", size as f64 / (1024.0 * 1024.0)),
            magic
        ),
    );
    let _ = window.emit("stt_log", format!("STT language: {}", lang));
    validate_model_file(&model_path)?;

    let data = base64::engine::general_purpose::STANDARD
        .decode(wav_base64.as_bytes())
        .map_err(|e| format!("Failed to decode audio: {}", e))?;

    let tmp_dir = Path::new(&workspace_path).join(".epris").join("tmp");
    std::fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let wav_path = tmp_dir.join(format!("stt-{}.wav", uuid::Uuid::new_v4()));
    std::fs::write(&wav_path, data).map_err(|e| e.to_string())?;

    let mut cmd = tokio::process::Command::new(&bin);
    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let task = selected_task(&app_handle)?.to_ascii_lowercase();
    let caps = detect_whisper_cli_caps(&bin).await.unwrap_or_default();
    let _ = window.emit(
        "stt_log",
        format!(
            "STT mode: {} (caps: --task={}, --translate={}, -tr={})",
            task, caps.supports_task, caps.supports_translate, caps.supports_tr_short
        ),
    );
    let mut chosen: Vec<&'static str> = Vec::new();
    if task == "translate" {
        if caps.supports_task {
            cmd.arg("--task").arg("translate");
            chosen.push("--task translate");
        } else if caps.supports_translate {
            cmd.arg("--translate");
            chosen.push("--translate");
        } else if caps.supports_tr_short {
            cmd.arg("-tr");
            chosen.push("-tr");
        }
    } else if caps.supports_task {
        // Force transcribe if supported, to avoid CLIs that default to translate.
        cmd.arg("--task").arg("transcribe");
        chosen.push("--task transcribe");
    }
    if !chosen.is_empty() {
        let _ = window.emit("stt_log", format!("whisper.cpp flags: {}", chosen.join(" ")));
    } else {
        let _ = window.emit("stt_log", "whisper.cpp flags: (none)".to_string());
    }
    let mut child = cmd
        .arg("-m")
        .arg(&model_path)
        .arg("-f")
        .arg(&wav_path)
        .arg("-l")
        .arg(lang)
        .current_dir(&workspace_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start whisper.cpp: {}", e))?;

    STT_CURRENT_PID.store(child.id().unwrap_or(0), Ordering::SeqCst);

    let stdout = child.stdout.take().ok_or("Failed to capture whisper stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture whisper stderr")?;
    let mut stdout_reader = tokio::io::BufReader::new(stdout).lines();
    let mut stderr_reader = tokio::io::BufReader::new(stderr).lines();

    #[derive(Clone, Copy)]
    enum StreamSource {
        Stdout,
        Stderr,
    }

    let (tx, mut rx) = mpsc::unbounded_channel::<(StreamSource, String)>();

    let tx_out = tx.clone();
    let stdout_task = tokio::spawn(async move {
        while let Ok(Some(line)) = stdout_reader.next_line().await {
            let _ = tx_out.send((StreamSource::Stdout, line));
        }
    });

    let tx_err = tx.clone();
    let stderr_task = tokio::spawn(async move {
        while let Ok(Some(line)) = stderr_reader.next_line().await {
            let _ = tx_err.send((StreamSource::Stderr, line));
        }
    });

    drop(tx);

    let is_deprecation_warning = |lower: &str| -> bool {
        (lower.starts_with("warning:") && lower.contains("deprecated"))
            || lower.contains("deprecation-warning")
            || (lower.contains("whisper-cli") && lower.contains("deprecated"))
            || (lower.contains("whisper-whisper-cli") && lower.contains("please use"))
    };

    enum LineKind {
        Transcript(String),
        Log(String),
        Ignore,
    }

    let classify_line = |trimmed: &str| -> LineKind {
        if trimmed.is_empty() {
            return LineKind::Ignore;
        }
        let lower = trimmed.to_ascii_lowercase();

        if is_deprecation_warning(&lower) {
            return LineKind::Log(trimmed.to_string());
        }

        // Treat obvious errors/warnings as logs, not transcript.
        if lower.starts_with("error:")
            || lower.starts_with("warning:")
            || lower.contains("failed to initialize whisper context")
        {
            return LineKind::Log(trimmed.to_string());
        }

        if let Some((_ts, rest)) = trimmed.rsplit_once(']') {
            let seg = rest.trim();
            if !seg.is_empty() {
                return LineKind::Transcript(seg.to_string());
            }
        }

        // Fallback: allow plain text outputs (no timestamps).
        // Most whisper.cpp builds print initialization and timing lines without timestamps; treat
        // those as logs so they don't pollute the user's prompt, but still show up for debugging.
        let noisy_log = lower.starts_with("whisper_")
            || lower.starts_with("system_info")
            || lower.starts_with("main:")
            || lower.starts_with("init:")
            || lower.starts_with("ggml_")
            || lower.contains("whisper_print_timings");
        if noisy_log {
            LineKind::Log(trimmed.to_string())
        } else {
            LineKind::Transcript(trimmed.to_string())
        }
    };

    // Cross-stream dedupe: prefer stdout. We only suppress stderr transcript lines if the exact
    // same line appears on stdout within a short TTL. We do not dedupe within a single stream.
    let mut recent_stdout: HashMap<String, Instant> = HashMap::new();
    let mut pending_stderr: VecDeque<(String, Instant)> = VecDeque::new();
    let mut recent_logs: VecDeque<String> = VecDeque::new();
    let mut tick = tokio::time::interval(Duration::from_millis(50));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    const STDOUT_TTL: Duration = Duration::from_secs(2);
    const STDERR_GRACE: Duration = Duration::from_millis(150);

    loop {
        tokio::select! {
            maybe = rx.recv() => {
                let Some((src, line)) = maybe else { break; };
                let trimmed = line.trim().to_string();
                let kind = classify_line(&trimmed);

                match src {
                    StreamSource::Stdout => {
                        match kind {
                            LineKind::Transcript(chunk) => {
                                // Do not dedupe within stdout (preserve real repetition).
                                let _ = window.emit("stt_chunk", chunk.clone());
                                recent_stdout.insert(chunk, Instant::now());
                            }
                            LineKind::Log(line) => {
                                recent_logs.push_back(line.clone());
                                if recent_logs.len() > 200 {
                                    recent_logs.pop_front();
                                }
                                let _ = window.emit("stt_log", line);
                            }
                            LineKind::Ignore => {}
                        }
                    }
                    StreamSource::Stderr => {
                        match kind {
                            LineKind::Transcript(chunk) => {
                                // Don't emit immediately; give stdout a chance to provide the same line.
                                pending_stderr.push_back((chunk, Instant::now()));
                            }
                            LineKind::Log(line) => {
                                recent_logs.push_back(line.clone());
                                if recent_logs.len() > 200 {
                                    recent_logs.pop_front();
                                }
                                // Keep stderr logs for debugging.
                                let _ = window.emit("stt_log", line);
                            }
                            LineKind::Ignore => {}
                        }
                    }
                }
            }
            _ = tick.tick() => {
                let now = Instant::now();
                recent_stdout.retain(|_, t| now.duration_since(*t) < STDOUT_TTL);

                while let Some((chunk, t0)) = pending_stderr.front().cloned() {
                    if now.duration_since(t0) < STDERR_GRACE {
                        break;
                    }
                    // If stdout produced it, drop it.
                    if let Some(t_out) = recent_stdout.get(&chunk) {
                        if now.duration_since(*t_out) < STDOUT_TTL {
                            pending_stderr.pop_front();
                            continue;
                        }
                    }
                    // Otherwise, emit from stderr now.
                    let _ = window.emit("stt_chunk", chunk);
                    pending_stderr.pop_front();
                }
            }
        }
    }

    // Flush any remaining pending stderr lines (best-effort).
    for (chunk, _) in pending_stderr.into_iter() {
        if recent_stdout.contains_key(&chunk) {
            continue;
        }
        let _ = window.emit("stt_chunk", chunk);
    }

    let status = child.wait().await.map_err(|e| e.to_string())?;
    let _ = stdout_task.await;
    let _ = stderr_task.await;
    STT_CURRENT_PID.store(0, Ordering::SeqCst);

    if STT_CANCEL_REQUESTED.load(Ordering::SeqCst) {
        return Err("Canceled".to_string());
    }

    if !status.success() {
        #[cfg(target_os = "windows")]
        {
            if status.code() == Some(-1073741515) {
                return Err("whisper.cpp failed to start (missing DLL dependency, 0xC0000135). Re-run STT install to ensure all DLLs were copied; if it still fails, install Microsoft Visual C++ Redistributable (x64).".to_string());
            }
        }
        let mut msg = format!("whisper.cpp failed (exit code: {:?})", status.code());
        if !recent_logs.is_empty() {
            msg.push_str("\nLast whisper output:\n");
            for line in recent_logs.iter().rev().take(40).rev() {
                msg.push_str(line);
                msg.push('\n');
            }
        }
        return Err(msg);
    }

    Ok(String::new())
}
