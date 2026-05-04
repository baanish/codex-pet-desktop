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

/// Returns the first process whose name or command-line contains `pattern`.
///
/// We deliberately avoid shelling out to `pgrep`/`pidof`: subprocesses can
/// stall (e.g., system load, sandbox prompts) and the polling thread serializes
/// every adapter, so a wedged probe would freeze all state updates. `sysinfo`
/// reads procfs (Linux) / mach VM info (macOS) / NtQuerySystemInformation
/// (Windows) directly and returns in bounded, predictable time.
pub fn find_process_by_name(pattern: &str) -> Option<u32> {
    use sysinfo::{ProcessRefreshKind, RefreshKind, System};
    let sys = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    for proc_ in sys.processes().values() {
        let name = proc_.name().to_string_lossy();
        if name.contains(pattern) {
            return Some(proc_.pid().as_u32());
        }
        let cmd_match = proc_
            .cmd()
            .iter()
            .any(|s| s.to_string_lossy().contains(pattern));
        if cmd_match {
            return Some(proc_.pid().as_u32());
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
