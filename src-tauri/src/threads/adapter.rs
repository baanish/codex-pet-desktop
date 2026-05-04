use crate::types::ActiveThread;

pub trait ThreadAdapter: Send + Sync {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn is_installed(&self) -> bool;
    fn poll(&self) -> Vec<ActiveThread>;
}

pub fn is_pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    unsafe {
        // Sending signal 0 just probes existence + permission.
        kill(pid as i32, 0) == 0
    }
    #[cfg(windows)]
    {
        use sysinfo::{Pid, System};
        let mut s = System::new();
        s.refresh_process(Pid::from_u32(pid));
        s.process(Pid::from_u32(pid)).is_some()
    }
}

/// Returns true if `pid` is alive AND its process name or any cmd-line argument
/// has a path component containing `name_substr` (case-insensitive). Used to
/// defend against PID reuse: when a session file recorded `pid=42` for a
/// dead Claude session and the OS later assigns 42 to an unrelated process,
/// we reject the stale record instead of treating it as live.
pub fn is_pid_for_app(pid: u32, name_substr: &str) -> bool {
    use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};
    let sys = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    let Some(proc_) = sys.process(Pid::from_u32(pid)) else {
        return false;
    };
    let needle = name_substr.to_ascii_lowercase();
    let name = proc_.name().to_string_lossy().to_ascii_lowercase();
    if name.contains(&needle) {
        return true;
    }
    for arg in proc_.cmd() {
        let s = arg.to_string_lossy().to_ascii_lowercase();
        if s.contains(&needle) {
            return true;
        }
    }
    false
}

#[cfg(unix)]
extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

/// Returns the pid of a process whose executable / cmd-line path has a
/// component exactly equal to `pattern`. Skips:
///   - the current process (so `find_process_by_name("codex")` doesn't
///     match `codex-pet-desktop` itself),
///   - any process whose path lives inside a macOS app bundle
///     (`/Contents/MacOS/`). This is what was causing detection regressions:
///     the Codex Electron *GUI* at `/Applications/Codex.app/Contents/MacOS/codex`
///     was matching `pattern == "codex"` even when no codex CLI was running,
///     and the adapter would report a phantom session.
///
/// We deliberately avoid shelling out to `pgrep`/`pidof`: subprocesses can
/// stall (system load, sandbox prompts) and a wedged probe used to freeze
/// every other adapter. `sysinfo` reads procfs / mach VM info /
/// NtQuerySystemInformation directly with bounded latency.
pub fn find_process_by_name(pattern: &str) -> Option<u32> {
    use sysinfo::{ProcessRefreshKind, RefreshKind, System};
    let self_pid = std::process::id();
    let sys = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    for proc_ in sys.processes().values() {
        let pid = proc_.pid().as_u32();
        if pid == self_pid {
            continue;
        }
        if process_is_macos_gui_bundle(proc_) {
            continue;
        }

        // Exact match on the executable basename ("codex", "opencode").
        if proc_.name().to_string_lossy() == pattern {
            return Some(pid);
        }
        // Or any cmd-line argument has a path component exactly equal to
        // pattern. Matches `/.../node_modules/codex/bin.js` and
        // `/.../codex/dist/index.js` while rejecting `/.../codex-pet-desktop`.
        for arg in proc_.cmd() {
            let s = arg.to_string_lossy();
            for component in s.split(['/', '\\']) {
                if component == pattern {
                    return Some(pid);
                }
            }
        }
    }
    None
}

fn process_is_macos_gui_bundle(proc_: &sysinfo::Process) -> bool {
    let exe_in_bundle = proc_
        .exe()
        .map(|p| p.to_string_lossy().contains("/Contents/MacOS/"))
        .unwrap_or(false);
    if exe_in_bundle {
        return true;
    }
    // Fall back to argv[0] in case `exe()` isn't populated.
    if let Some(arg0) = proc_.cmd().first() {
        if arg0.to_string_lossy().contains("/Contents/MacOS/") {
            return true;
        }
    }
    false
}

/// Resolve the working directory of `pid`, if the OS will tell us. macOS
/// requires the calling process to have permission to introspect another
/// process; sysinfo wraps libproc on darwin and can return None for system
/// or sandboxed processes. That's fine — we just omit the cwd from the row.
pub fn process_cwd(pid: u32) -> Option<String> {
    use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};
    let sys = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    let p = sys.process(Pid::from_u32(pid))?;
    let cwd = p.cwd()?;
    Some(cwd.to_string_lossy().into_owned())
}

pub fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
