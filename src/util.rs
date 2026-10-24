use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

pub fn expand_home(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix('~') {
        let home = dirs::home_dir().unwrap_or_default();
        return home.join(rest.trim_start_matches('/'));
    }
    PathBuf::from(p)
}

pub fn short_path(p: &Path) -> String {
    let s = p.to_string_lossy();
    if let Some(home) = dirs::home_dir() {
        if let Ok(rest) = p.strip_prefix(&home) {
            return format!("~/{}", rest.to_string_lossy());
        }
    }
    s.into_owned()
}

/// Run a command with a timeout; None on any failure.
pub fn run(cmd: &str, args: &[&str]) -> Option<String> {
    run_in(cmd, args, None)
}

pub fn run_in(cmd: &str, args: &[&str], cwd: Option<&Path>) -> Option<String> {
    let mut c = Command::new(cmd);
    c.args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    if let Some(d) = cwd {
        c.current_dir(d);
    }
    let mut child = c.spawn().ok()?;
    let deadline = std::time::Instant::now() + Duration::from_secs(8);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let out = child.wait_with_output().ok()?;
                if !status.success() && out.stdout.is_empty() {
                    return None;
                }
                return Some(String::from_utf8_lossy(&out.stdout).trim().to_string());
            }
            Ok(None) if std::time::Instant::now() > deadline => {
                let _ = child.kill();
                return None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(15)),
            Err(_) => return None,
        }
    }
}

pub fn which(name: &str) -> Option<String> {
    run("which", &[name]).filter(|s| !s.is_empty())
}

pub fn read(p: &Path) -> Option<String> {
    std::fs::read_to_string(p).ok()
}

pub fn read_json(p: &Path) -> Option<serde_json::Value> {
    serde_json::from_str(&read(p)?).ok()
}

pub fn repo_id(dir: &Path) -> String {
    format!("repo:{}", dir.display())
}

pub fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Minimal UTC formatter to avoid a chrono dependency.
    let days = secs / 86400;
    let rem = secs % 86400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let (y, mo, d) = civil_from_days(days as i64);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}
