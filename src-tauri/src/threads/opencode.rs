use super::adapter::{find_process_by_name, now_ms, ThreadAdapter};
use crate::types::{ActiveThread, ThreadStatus};
use std::path::PathBuf;

const BUSY_THRESHOLD_MS: u64 = 10_000;

pub struct OpenCodeAdapter {
    db_path: PathBuf,
    wal_path: PathBuf,
}

impl OpenCodeAdapter {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_default();
        let data_dir = home.join(".local/share/opencode");
        Self {
            db_path: data_dir.join("opencode.db"),
            wal_path: data_dir.join("opencode.db-wal"),
        }
    }
}

impl ThreadAdapter for OpenCodeAdapter {
    fn id(&self) -> &str {
        "opencode"
    }

    fn display_name(&self) -> &str {
        "OpenCode"
    }

    fn is_installed(&self) -> bool {
        self.db_path.exists()
    }

    fn poll(&self) -> Vec<ActiveThread> {
        if !self.wal_path.exists() {
            return vec![];
        }
        let wal_size = std::fs::metadata(&self.wal_path)
            .map(|m| m.len())
            .unwrap_or(0);
        if wal_size == 0 {
            return vec![];
        }

        if find_process_by_name("opencode").is_none() {
            return vec![ActiveThread {
                tool: self.id().into(),
                status: ThreadStatus::Stale,
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
        let _ = conn.busy_timeout(std::time::Duration::from_millis(500));

        let row: Option<(Option<String>, i64)> = conn
            .query_row(
                "SELECT title, time_updated FROM session WHERE time_archived IS NULL ORDER BY time_updated DESC LIMIT 1",
                [],
                |r| Ok((r.get::<_, Option<String>>(0)?, r.get::<_, i64>(1)?)),
            )
            .ok();

        let Some((title, time_updated)) = row else {
            return vec![ActiveThread {
                tool: self.id().into(),
                status: ThreadStatus::Waiting,
                title: None,
            }];
        };

        // Same reasoning as the codex adapter: stale `time_updated` while the
        // opencode process is alive means a long-running step, not an idle
        // session. Use `waiting` so the row stays visible.
        let age = now_ms().saturating_sub(time_updated as u64);
        vec![ActiveThread {
            tool: self.id().into(),
            status: if age < BUSY_THRESHOLD_MS {
                ThreadStatus::Busy
            } else {
                ThreadStatus::Waiting
            },
            title,
        }]
    }
}
