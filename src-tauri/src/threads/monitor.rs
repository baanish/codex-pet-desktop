use super::adapter::ThreadAdapter;
use crate::types::{ActiveThread, ThreadStatus};
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

pub struct ThreadMonitor {
    adapters: Vec<Arc<dyn ThreadAdapter>>,
    enabled: Arc<Mutex<HashMap<String, bool>>>,
    interval_ms: Arc<Mutex<u64>>,
    callback: Arc<dyn Fn(Vec<ActiveThread>) + Send + Sync>,
    wake: Arc<(parking_lot::Mutex<bool>, parking_lot::Condvar)>,
    running: Arc<Mutex<bool>>,
    // Adapters currently being polled in a worker thread. Used as a
    // single-flight guard so a wedged adapter cannot accumulate orphan
    // threads on every interval.
    in_flight: Arc<Mutex<HashSet<String>>>,
    // Last successful (or last-emitted) results per adapter. While an adapter
    // is wedged we keep replaying its previous state so the renderer doesn't
    // see the thread vanish, and surface a synthetic stale entry instead.
    last_results: Arc<Mutex<HashMap<String, Vec<ActiveThread>>>>,
    // Runtime-only dismissals. Keys are pruned as soon as the corresponding
    // thread is no longer detected so a future, unrelated session cannot
    // inherit a stale hidden state.
    dismissed: Arc<Mutex<HashSet<String>>>,
    last_detected: Arc<Mutex<Vec<ActiveThread>>>,
}

impl ThreadMonitor {
    pub fn new(
        adapters: Vec<Arc<dyn ThreadAdapter>>,
        callback: impl Fn(Vec<ActiveThread>) + Send + Sync + 'static,
    ) -> Self {
        Self {
            adapters: adapters.into_iter().filter(|a| a.is_installed()).collect(),
            enabled: Arc::new(Mutex::new(HashMap::new())),
            interval_ms: Arc::new(Mutex::new(30_000)),
            callback: Arc::new(callback),
            wake: Arc::new((parking_lot::Mutex::new(false), parking_lot::Condvar::new())),
            running: Arc::new(Mutex::new(false)),
            in_flight: Arc::new(Mutex::new(HashSet::new())),
            last_results: Arc::new(Mutex::new(HashMap::new())),
            dismissed: Arc::new(Mutex::new(HashSet::new())),
            last_detected: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn list_adapters(&self) -> Vec<(String, String)> {
        self.adapters
            .iter()
            .map(|a| (a.id().to_string(), a.display_name().to_string()))
            .collect()
    }

    pub fn set_enabled(&self, map: HashMap<String, bool>) {
        *self.enabled.lock() = map;
    }

    pub fn set_interval(&self, ms: u64) {
        *self.interval_ms.lock() = ms;
        self.wake_now();
    }

    pub fn trigger(&self) {
        self.wake_now();
    }

    pub fn dismiss_thread(&self, thread: ActiveThread) {
        if !is_dismissible(&thread) {
            return;
        }
        self.dismissed.lock().insert(thread_key(&thread));
        let visible = self.filter_dismissed(self.last_detected.lock().clone());
        (self.callback)(visible);
    }

    fn wake_now(&self) {
        let (lock, cvar) = &*self.wake;
        let mut g = lock.lock();
        *g = true;
        cvar.notify_all();
    }

    pub fn start(self: &Arc<Self>) {
        {
            let mut r = self.running.lock();
            if *r {
                return;
            }
            *r = true;
        }
        let me = self.clone();
        thread::spawn(move || me.run_loop());
    }

    fn run_loop(self: Arc<Self>) {
        // Initial poll immediately
        self.poll_once();
        loop {
            let interval = *self.interval_ms.lock();
            let (lock, cvar) = &*self.wake;
            let mut g = lock.lock();
            let deadline = Instant::now() + Duration::from_millis(interval);
            while !*g {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    break;
                }
                cvar.wait_for(&mut g, remaining);
            }
            *g = false;
            drop(g);

            if !*self.running.lock() {
                break;
            }
            self.poll_once();
        }
    }

    fn poll_once(&self) {
        // Each adapter polls in its own worker thread with:
        //   - a single-flight guard (skip an adapter that's still pending
        //     from the previous interval — keeps the orphan-thread count
        //     bounded at <= 1 per adapter regardless of poll cadence),
        //   - a hard 5-second wall-clock budget across the sweep, so a
        //     stalled adapter cannot freeze the others, and
        //   - a stale-replay fallback so the renderer's view of a wedged
        //     adapter degrades to "previously seen, now stale" instead of
        //     silently disappearing.
        use std::sync::mpsc;

        let enabled = self.enabled.lock().clone();
        let mut started: Vec<(String, mpsc::Receiver<Vec<ActiveThread>>)> = vec![];

        for a in &self.adapters {
            let id = a.id().to_string();
            if matches!(enabled.get(&id), Some(false)) {
                continue;
            }
            // Single-flight: if the previous poll for this adapter hasn't
            // returned yet, skip launching another. We'll replay last_results
            // for it below and try again next interval.
            {
                let mut g = self.in_flight.lock();
                if g.contains(&id) {
                    continue;
                }
                g.insert(id.clone());
            }

            let adapter = a.clone();
            let in_flight = self.in_flight.clone();
            let last = self.last_results.clone();
            let id_for_thread = id.clone();
            let (tx, rx) = mpsc::channel();
            thread::spawn(move || {
                let result = adapter.poll();
                last.lock().insert(id_for_thread.clone(), result.clone());
                in_flight.lock().remove(&id_for_thread);
                let _ = tx.send(result);
            });
            started.push((id, rx));
        }

        let deadline = Instant::now() + Duration::from_secs(5);
        let mut by_adapter: HashMap<String, Vec<ActiveThread>> = HashMap::new();
        for (id, rx) in started {
            let remaining = deadline.saturating_duration_since(Instant::now());
            match rx.recv_timeout(remaining) {
                Ok(threads) => {
                    by_adapter.insert(id, threads);
                }
                Err(_) => {
                    // Worker still hasn't returned within budget; in_flight
                    // stays set so we won't spawn another for this adapter
                    // until it completes.
                }
            }
        }

        // Build the full state by merging fresh results with replayed
        // last_results for adapters that didn't return in time. Replayed
        // entries are downgraded to `stale`.
        let mut all = Vec::new();
        let last = self.last_results.lock();
        for a in &self.adapters {
            let id = a.id();
            if matches!(enabled.get(id), Some(false)) {
                continue;
            }
            if let Some(fresh) = by_adapter.remove(id) {
                all.extend(fresh);
            } else if let Some(prev) = last.get(id) {
                for t in prev {
                    all.push(ActiveThread {
                        tool: t.tool.clone(),
                        status: ThreadStatus::Stale,
                        title: t.title.clone(),
                        cwd: t.cwd.clone(),
                        pid: t.pid,
                    });
                }
            }
        }
        drop(last);

        *self.last_detected.lock() = all.clone();
        let visible = self.filter_dismissed(all);
        (self.callback)(visible);
    }

    fn filter_dismissed(&self, threads: Vec<ActiveThread>) -> Vec<ActiveThread> {
        let live_keys: HashSet<String> = threads.iter().map(thread_key).collect();
        let mut dismissed = self.dismissed.lock();
        dismissed.retain(|key| live_keys.contains(key));
        threads
            .into_iter()
            .filter(|thread| !dismissed.contains(&thread_key(thread)))
            .collect()
    }
}

fn is_dismissible(thread: &ActiveThread) -> bool {
    !matches!(thread.status, ThreadStatus::Busy)
}

fn thread_key(thread: &ActiveThread) -> String {
    format!(
        "{}\n{}\n{}\n{}",
        thread.tool,
        thread.pid.map(|pid| pid.to_string()).unwrap_or_default(),
        thread.cwd.as_deref().unwrap_or_default(),
        thread.title.as_deref().unwrap_or_default()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thread(status: ThreadStatus, pid: Option<u32>, title: &str) -> ActiveThread {
        ActiveThread {
            tool: "codex".into(),
            status,
            title: Some(title.into()),
            cwd: Some("/tmp/project".into()),
            pid,
        }
    }

    #[test]
    fn only_non_busy_threads_can_be_dismissed() {
        assert!(!is_dismissible(&thread(
            ThreadStatus::Busy,
            Some(1),
            "work"
        )));
        assert!(is_dismissible(&thread(
            ThreadStatus::Waiting,
            Some(1),
            "work"
        )));
        assert!(is_dismissible(&thread(
            ThreadStatus::Error,
            Some(1),
            "work"
        )));
        assert!(is_dismissible(&thread(ThreadStatus::Idle, Some(1), "work")));
        assert!(is_dismissible(&thread(
            ThreadStatus::Stale,
            Some(1),
            "work"
        )));
    }

    #[test]
    fn dismissed_keys_are_pruned_when_threads_fall_out_of_detection() {
        let monitor = ThreadMonitor::new(Vec::new(), |_| {});
        let first = thread(ThreadStatus::Waiting, Some(10), "old");
        let second = thread(ThreadStatus::Waiting, Some(11), "new");
        monitor.dismissed.lock().insert(thread_key(&first));

        let visible = monitor.filter_dismissed(vec![second.clone()]);

        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].pid, Some(11));
        assert!(monitor.dismissed.lock().is_empty());
    }
}
