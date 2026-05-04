use super::adapter::{find_process_by_name, now_ms, ThreadAdapter};
use crate::types::{ActiveThread, ThreadStatus};
use std::path::PathBuf;

const BUSY_THRESHOLD_MS: u64 = 10_000;

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
        // Liveness via sysinfo only — the legacy implementation also tried lsof
        // on each lockfile under tmp/arg0/codex-*/.lock, but that's a
        // synchronous shell-out per directory and the monitor loop is
        // serialized. A blocking `lsof` would freeze every adapter. The
        // process-name probe is enough to know if codex is currently running.
        if find_process_by_name("codex").is_none() {
            return vec![];
        }

        if !self.db_path.exists() {
            return vec![ActiveThread {
                tool: self.id().into(),
                status: ThreadStatus::Busy,
                title: None,
            }];
        }

        let conn = match rusqlite::Connection::open_with_flags(
            &self.db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        ) {
            Ok(c) => c,
            Err(_) => return vec![],
        };

        let row: Option<(Option<String>, i64)> = conn
            .query_row(
                "SELECT title, updated_at FROM threads WHERE archived=0 ORDER BY updated_at DESC LIMIT 1",
                [],
                |r| Ok((r.get::<_, Option<String>>(0)?, r.get::<_, i64>(1)?)),
            )
            .ok();

        let Some((title, updated_at)) = row else {
            return vec![ActiveThread {
                tool: self.id().into(),
                status: ThreadStatus::Idle,
                title: None,
            }];
        };

        // updated_at is seconds, like the original
        let age = now_ms().saturating_sub((updated_at as u64) * 1000);
        vec![ActiveThread {
            tool: self.id().into(),
            status: if age < BUSY_THRESHOLD_MS {
                ThreadStatus::Busy
            } else {
                ThreadStatus::Idle
            },
            title,
        }]
    }
}
