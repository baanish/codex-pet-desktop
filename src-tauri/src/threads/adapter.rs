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

#[cfg(unix)]
extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

/// Returns the pid of a process whose executable / cmd-line path has a
/// component exactly equal to `pattern`. The current process is always
/// excluded — without that guard, calling this with `"codex"` would match
/// our own `codex-pet-desktop` binary (substring match) and the pet adapter
/// would happily report itself as live Codex activity.
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

pub fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
