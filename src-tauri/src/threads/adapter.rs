use crate::types::ActiveThread;
use std::process::Command;

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
        libc_kill(pid as i32, 0) == 0
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

#[cfg(unix)]
unsafe fn libc_kill(pid: i32, sig: i32) -> i32 {
    kill(pid, sig)
}

pub fn find_process_by_name(pattern: &str) -> Option<u32> {
    // pgrep is available on macOS + Linux; on Windows fall back to sysinfo.
    #[cfg(unix)]
    {
        if let Ok(output) = Command::new("pgrep").arg("-f").arg(pattern).output() {
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout);
                if let Some(first) = s.lines().next() {
                    if let Ok(pid) = first.trim().parse::<u32>() {
                        return Some(pid);
                    }
                }
            }
        }
    }

    use sysinfo::{ProcessRefreshKind, RefreshKind, System};
    let sys = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    for proc_ in sys.processes().values() {
        let name = proc_.name().to_string_lossy();
        let cmd_match = proc_
            .cmd()
            .iter()
            .any(|s| s.to_string_lossy().contains(pattern));
        if name.contains(pattern) || cmd_match {
            return Some(proc_.pid().as_u32());
        }
    }
    None
}

pub fn is_lock_held_by_process(lock_path: &str) -> bool {
    // lsof is macOS/Linux; we don't try to emulate on Windows.
    #[cfg(unix)]
    {
        if let Ok(output) = Command::new("lsof").arg(lock_path).output() {
            if !output.stdout.is_empty() {
                return true;
            }
        }
    }
    false
}

pub fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
