use super::adapter::{is_pid_alive, is_pid_for_app, now_ms, process_cwd, ThreadAdapter};
use crate::types::{ActiveThread, ThreadStatus};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
struct SessionFile {
    pid: Option<u32>,
    name: Option<String>,
    status: Option<String>,
}

pub struct ClaudeCodeAdapter {
    sessions_dir: PathBuf,
    claude_root: PathBuf,
}

impl ClaudeCodeAdapter {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_default();
        Self {
            claude_root: home.join(".claude"),
            sessions_dir: home.join(".claude/sessions"),
        }
    }
}

impl ThreadAdapter for ClaudeCodeAdapter {
    fn id(&self) -> &str {
        "claude-code"
    }

    fn display_name(&self) -> &str {
        "Claude Code"
    }

    fn is_installed(&self) -> bool {
        self.claude_root.exists()
    }

    fn poll(&self) -> Vec<ActiveThread> {
        if !self.sessions_dir.exists() {
            return vec![];
        }

        let mut active = Vec::new();
        let entries = match std::fs::read_dir(&self.sessions_dir) {
            Ok(e) => e,
            Err(_) => return vec![],
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let raw = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let data: SessionFile = match serde_json::from_str(&raw) {
                Ok(d) => d,
                Err(_) => continue,
            };

            if let Some(pid) = data.pid {
                // PID reuse defense: pid alive alone isn't enough — if Claude
                // exited and the OS handed pid to an unrelated process, the
                // session file would otherwise look "live" forever. Confirm
                // the process is actually a Claude one before trusting it.
                if is_pid_alive(pid) && is_pid_for_app(pid, "claude") {
                    let status = if data.status.as_deref() == Some("busy") {
                        ThreadStatus::Busy
                    } else {
                        ThreadStatus::Idle
                    };
                    active.push(ActiveThread {
                        tool: self.id().into(),
                        status,
                        title: data.name,
                        cwd: process_cwd(pid),
                        pid: Some(pid),
                    });
                    continue;
                }
            }

            // Dead PID (or PID-reuse-mismatch) → drop the entry entirely
            // instead of surfacing a `stale` row. The user only wants
            // active threads; a session JSON whose process exited isn't
            // active. Adapter-wedge cases are handled by the monitor's
            // stale-replay separately.
            let _ = now_ms;
        }

        active
    }
}
