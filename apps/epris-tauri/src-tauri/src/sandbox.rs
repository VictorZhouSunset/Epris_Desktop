use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};
use walkdir::WalkDir;

pub const AUDIT_ALLOWED_TOP_LEVEL_DIRS: &[&str] = &["src", "public", ".gemini", ".opencode"];
pub const AUDIT_IGNORE_TOP_LEVEL_DIRS: &[&str] = &["logs", ".epris"];
pub const AUDIT_FORBIDDEN_TOP_LEVEL_DIRS: &[&str] = &["node_modules", ".git"];

#[derive(Debug, Clone)]
pub struct AuditReport {
    pub violations: Vec<String>,
    pub incomplete: bool,
}

fn is_after(maybe_modified: std::io::Result<SystemTime>, since: SystemTime) -> bool {
    match maybe_modified {
        Ok(ts) => ts.duration_since(since).is_ok(),
        Err(_) => false,
    }
}

fn find_recent_write_in_dir(
    root: &Path,
    since: SystemTime,
    max_entries: usize,
    max_duration: Duration,
) -> (Option<PathBuf>, bool) {
    let started = Instant::now();
    let mut visited = 0usize;

    for entry in WalkDir::new(root).follow_links(false).into_iter().filter_map(Result::ok) {
        visited += 1;
        if visited > max_entries || started.elapsed() > max_duration {
            return (None, true);
        }

        let path = entry.path();
        if entry.file_type().is_file() {
            let modified = std::fs::metadata(path).and_then(|m| m.modified());
            if is_after(modified, since) {
                return (Some(path.to_path_buf()), false);
            }
        }
    }

    (None, false)
}

fn rel_string(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

pub fn audit_workspace_after_ai_run(workspace_root: &Path, since: SystemTime) -> Result<AuditReport, String> {
    let mut violations: Vec<String> = Vec::new();
    let mut incomplete = false;

    // 1) Forbidden directories: any write here is a violation.
    for dir in AUDIT_FORBIDDEN_TOP_LEVEL_DIRS {
        let p = workspace_root.join(dir);
        if !p.exists() {
            continue;
        }

        // node_modules can be huge; keep it best-effort with a tight time budget.
        let (max_entries, max_duration) = if *dir == "node_modules" {
            (20_000usize, Duration::from_millis(250))
        } else {
            (50_000usize, Duration::from_secs(2))
        };

        let (hit, timed_out) = find_recent_write_in_dir(&p, since, max_entries, max_duration);
        if timed_out {
            incomplete = true;
        }
        if let Some(h) = hit {
            violations.push(rel_string(workspace_root, &h));
        }
    }

    // 2) Disallowed paths: any write outside allowed dirs (excluding app-internal dirs) is a violation.
    for entry in std::fs::read_dir(workspace_root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();

        if AUDIT_ALLOWED_TOP_LEVEL_DIRS.iter().any(|d| *d == name) {
            continue;
        }
        if AUDIT_IGNORE_TOP_LEVEL_DIRS.iter().any(|d| *d == name) {
            continue;
        }
        if AUDIT_FORBIDDEN_TOP_LEVEL_DIRS.iter().any(|d| *d == name) {
            continue;
        }

        let path = entry.path();
        let ft = entry.file_type().map_err(|e| e.to_string())?;

        if ft.is_file() {
            let modified = std::fs::metadata(&path)
                .and_then(|m| m.modified())
                .map_err(|e| e.to_string())?;
            if modified.duration_since(since).is_ok() {
                violations.push(rel_string(workspace_root, &path));
            }
            continue;
        }

        if ft.is_dir() {
            let (hit, timed_out) =
                find_recent_write_in_dir(&path, since, 20_000usize, Duration::from_secs(1));
            if timed_out {
                incomplete = true;
            }
            if let Some(h) = hit {
                violations.push(rel_string(workspace_root, &h));
            }
        }
    }

    violations.sort();
    violations.dedup();

    Ok(AuditReport { violations, incomplete })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn touch_after(path: &Path, since: SystemTime) {
        // Ensure file mtime is after `since` even on coarse timestamp file systems.
        while SystemTime::now().duration_since(since).unwrap_or_default() < Duration::from_millis(10) {
            std::thread::sleep(Duration::from_millis(5));
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, "x").unwrap();
    }

    #[test]
    fn allows_writes_in_src() {
        let tmp = tempdir().unwrap();
        let ws = tmp.path();
        std::fs::create_dir_all(ws.join("src")).unwrap();

        let since = SystemTime::now();
        touch_after(&ws.join("src").join("a.tsx"), since);

        let report = audit_workspace_after_ai_run(ws, since).unwrap();
        assert!(report.violations.is_empty());
    }

    #[test]
    fn allows_writes_in_gemini_dir() {
        let tmp = tempdir().unwrap();
        let ws = tmp.path();
        std::fs::create_dir_all(ws.join(".gemini")).unwrap();

        let since = SystemTime::now();
        touch_after(&ws.join(".gemini").join("settings.json"), since);

        let report = audit_workspace_after_ai_run(ws, since).unwrap();
        assert!(report.violations.is_empty());
    }

    #[test]
    fn ignores_logs_and_epris_dirs() {
        let tmp = tempdir().unwrap();
        let ws = tmp.path();
        std::fs::create_dir_all(ws.join("logs")).unwrap();
        std::fs::create_dir_all(ws.join(".epris")).unwrap();

        let since = SystemTime::now();
        touch_after(&ws.join("logs").join("opencode.log"), since);
        touch_after(&ws.join(".epris").join("HEAD"), since);

        let report = audit_workspace_after_ai_run(ws, since).unwrap();
        assert!(report.violations.is_empty());
    }

    #[test]
    fn flags_writes_in_node_modules() {
        let tmp = tempdir().unwrap();
        let ws = tmp.path();
        std::fs::create_dir_all(ws.join("node_modules")).unwrap();

        let since = SystemTime::now();
        touch_after(&ws.join("node_modules").join("x.txt"), since);

        let report = audit_workspace_after_ai_run(ws, since).unwrap();
        assert!(
            report.violations.iter().any(|p| p.starts_with("node_modules/") || p == "node_modules\\x.txt" || p == "node_modules/x.txt"),
            "expected node_modules write violation, got: {:?}",
            report.violations
        );
    }

    #[test]
    fn flags_writes_outside_allowed_dirs() {
        let tmp = tempdir().unwrap();
        let ws = tmp.path();

        let since = SystemTime::now();
        touch_after(&ws.join("README.md"), since);

        let report = audit_workspace_after_ai_run(ws, since).unwrap();
        assert!(report.violations.iter().any(|p| p == "README.md"));
    }

    #[test]
    fn flags_writes_in_git_dir() {
        let tmp = tempdir().unwrap();
        let ws = tmp.path();
        std::fs::create_dir_all(ws.join(".git")).unwrap();

        let since = SystemTime::now();
        touch_after(&ws.join(".git").join("config"), since);

        let report = audit_workspace_after_ai_run(ws, since).unwrap();
        assert!(
            report.violations.iter().any(|p| p.starts_with(".git/") || p.starts_with(".git\\")),
            "expected .git write violation, got: {:?}",
            report.violations
        );
    }
}

