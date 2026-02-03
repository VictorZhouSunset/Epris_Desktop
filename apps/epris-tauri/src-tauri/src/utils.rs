use std::path::Path;
#[cfg(any(test, debug_assertions))]
use std::path::PathBuf;
use std::process::{Child, Command};
use std::net::TcpListener;

pub const SNAPSHOT_WHITELIST: &[&str] = &["src", "public"];
pub const FORBIDDEN_FILES: &[&str] = &[
    "package.json",
    "pnpm-lock.yaml",
    "tsconfig.json",
    "vite.config.ts",
    "index.html"
];

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// Searches upward from the given path to find the project root containing the anchor.
///
/// NOTE: This is only used in dev builds to find repo-relative resources.
/// Release builds use Tauri resources / embedded templates instead.
#[cfg(any(test, debug_assertions))]
pub fn find_project_root(start_path: &Path, anchor: &str) -> Option<PathBuf> {
    let mut current = start_path.to_path_buf();
    loop {
        if current.join(anchor).exists() {
            return Some(current);
        }
        if !current.pop() {
            break;
        }
    }
    None
}

pub fn is_port_available(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

pub fn find_available_port(start_port: u16) -> u16 {
    let mut port = start_port;
    while !is_port_available(port) && port < 65535 {
        port += 1;
        // Minor throttle to prevent hammering
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    port
}

pub fn kill_process_tree(child: &mut Child) {
    let pid = child.id();
    println!("[Epris] Killing process tree for PID: {}", pid);
    
    #[cfg(target_os = "windows")]
    {
        // On Windows, use taskkill to kill the entire process tree (/T)
        let _ = Command::new("taskkill")
            .args(&["/F", "/T", "/PID", &pid.to_string()])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .status();
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        let _ = child.kill();
    }
}

pub fn kill_process_on_port(port: u16) {
    println!("[Epris] Checking for process on port: {}", port);
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("cmd")
            .args(&["/C", &format!("netstat -ano | findstr :{}", port)])
            .output();
        
        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                if line.contains("LISTENING") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(pid_str) = parts.last() {
                        println!("[Epris] Killing lingering process on port {}: PID {}", port, pid_str);
                        let _ = Command::new("taskkill")
                            .args(&["/F", "/PID", pid_str, "/T"])
                            .creation_flags(0x08000000)
                            .status();
                    }
                }
            }
        }
    }
}

pub fn create_shell_command(program: &str, args: &[&str]) -> Command {
    #[cfg(target_os = "windows")]
    {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C");
        cmd.arg(program);
        cmd.args(args);
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        cmd
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut cmd = Command::new(program);
        cmd.args(args);
        cmd
    }
}

pub fn create_async_shell_command(program: &str, args: &[&str]) -> tokio::process::Command {
    #[cfg(target_os = "windows")]
    {
        let mut cmd = tokio::process::Command::new("cmd");
        cmd.arg("/C");
        cmd.arg(program);
        cmd.args(args);
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        cmd
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut cmd = tokio::process::Command::new(program);
        cmd.args(args);
        cmd
    }
}

pub fn generate_line_diff(old_text: &str, new_text: &str) -> String {
    let old_lines: Vec<&str> = old_text.lines().collect();
    let new_lines: Vec<&str> = new_text.lines().collect();
    let mut diff = String::new();
    
    let max_len = std::cmp::max(old_lines.len(), new_lines.len());
    for i in 0..max_len {
        let old = old_lines.get(i);
        let new = new_lines.get(i);
        
        match (old, new) {
            (Some(o), Some(n)) if o != n => {
                diff.push_str(&format!("- L{}: {}\n", i + 1, o));
                diff.push_str(&format!("+ L{}: {}\n", i + 1, n));
            }
            (Some(o), None) => {
                diff.push_str(&format!("- L{}: {}\n", i + 1, o));
            }
            (None, Some(n)) => {
                diff.push_str(&format!("+ L{}: {}\n", i + 1, n));
            }
            _ => {} // No change or identical
        }
    }
    diff
}

pub fn check_forbidden_files(
    workspace_path: &str,
    compare_base_path: &Path,
) -> Result<Vec<String>, String> {
    let mut modified_forbidden = Vec::new();

    for forbidden_file in FORBIDDEN_FILES {
        let workspace_file = Path::new(workspace_path).join(forbidden_file);
        let compare_file = compare_base_path.join(forbidden_file);

        if workspace_file.exists() && compare_file.exists() {
            let workspace_content = std::fs::read(&workspace_file)
                .map_err(|e| format!("Failed to read {}: {}", forbidden_file, e))?;
            let compare_content = std::fs::read(&compare_file)
                .map_err(|e| format!("Failed to read baseline {}: {}", forbidden_file, e))?;

            if workspace_content != compare_content {
                modified_forbidden.push(forbidden_file.to_string());
            }
        } else if workspace_file.exists() != compare_file.exists() {
            modified_forbidden.push(forbidden_file.to_string());
        }
    }

    Ok(modified_forbidden)
}
