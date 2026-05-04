use super::adapter::{find_process_by_name, now_ms, process_cwd, ThreadAdapter};
use crate::types::{ActiveThread, ThreadStatus};
use std::path::{Path, PathBuf};

const BUSY_THRESHOLD_MS: u64 = 10_000;
const SESSION_DIR_MAX_AGE_SECS: u64 = 3600;

fn has_recent_codex_session_dir(locks_dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(locks_dir) else {
        return false;
    };
    for entry in entries.flatten() {
        if !entry
            .file_name()
            .to_string_lossy()
            .starts_with("codex-")
        {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let Ok(modified) = meta.modified() else { continue };
        if let Ok(elapsed) = modified.elapsed() {
            if elapsed.as_secs() <= SESSION_DIR_MAX_AGE_SECS {
                return true;
            }
        }
    }
    false
}

pub struct CodexAdapter {
    codex_dir: PathBuf,
    db_path: PathBuf,
}

impl CodexAdapter {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_default();
        let codex_dir = home.join(".codex");
        let db_path = codex_dir.join("state_5.sqlite");
        Self { codex_dir, db_path }
    }
}

impl ThreadAdapter for CodexAdapter {
    fn id(&self) -> &str {
        "codex"
    }

    fn display_name(&self) -> &str {
        "Codex"
    }

    fn is_installed(&self) -> bool {
        self.codex_dir.exists()
    }

    fn poll(&self) -> Vec<ActiveThread> {
        // Two independent liveness signals must both fire:
        //   (a) at least one ~/.codex/tmp/arg0/codex-*/ session directory
        //       exists and was touched in the last hour, and
        //   (b) a process matching "codex" is currently alive.
        // Either signal alone is too loose: an unrelated process named
        // `codex` (cycle-8 finding) would otherwise resurrect a stale row,
        // and a leftover session directory after a crash would otherwise
        // outlive its session forever. Skip the lsof shell-out (cycle 2);
        // the directory mtime is enough of a session-level marker.
        let locks_dir = self.codex_dir.join("tmp/arg0");
        if !has_recent_codex_session_dir(&locks_dir) {
            return vec![];
        }
        let Some(pid) = find_process_by_name("codex") else {
            return vec![];
        };
        let cwd = process_cwd(pid);

        if !self.db_path.exists() {
            return vec![ActiveThread {
                tool: self.id().into(),
                status: ThreadStatus::Busy,
                title: None,
                cwd,
                pid: Some(pid),
            }];
        }

        let conn = match rusqlite::Connection::open_with_flags(
            &self.db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        ) {
            Ok(c) => c,
            Err(_) => return vec![],
        };
        // If another process holds an exclusive lock, fail fast rather than
        // spin until the monitor's wall-clock budget elapses.
        let _ = conn.busy_timeout(std::time::Duration::from_millis(500));

        let row: Option<(Option<String>, i64)> = conn
            .query_row(
                "SELECT title, updated_at FROM threads WHERE archived=0 ORDER BY updated_at DESC LIMIT 1",
                [],
                |r| Ok((r.get::<_, Option<String>>(0)?, r.get::<_, i64>(1)?)),
            )
            .ok();

        let Some((title, updated_at)) = row else {
            // Process is alive but no thread row yet; treat as waiting for
            // input so the user sees the session rather than having it
            // filtered out as `idle`.
            return vec![ActiveThread {
                tool: self.id().into(),
                status: ThreadStatus::Waiting,
                title: None,
                cwd,
                pid: Some(pid),
            }];
        };

        // The DB writes are bursty; "fresh" updated_at means the agent is
        // actively touching the thread, but a stale timestamp does NOT mean
        // the session is idle in the user-visible sense — codex just hasn't
        // flushed in a while during a long tool call. So we fall back to
        // `waiting` (visible, dimmed) instead of `idle` (filtered out).
        let age = now_ms().saturating_sub((updated_at as u64) * 1000);
        vec![ActiveThread {
            tool: self.id().into(),
            status: if age < BUSY_THRESHOLD_MS {
                ThreadStatus::Busy
            } else {
                ThreadStatus::Waiting
            },
            title,
            cwd,
            pid: Some(pid),
        }]
    }
}
