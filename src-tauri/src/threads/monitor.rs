use super::adapter::ThreadAdapter;
use crate::types::ActiveThread;
use parking_lot::Mutex;
use std::collections::HashMap;
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
        // Each adapter polls in its own thread with a hard wall-clock budget,
        // so a wedged sqlite open or filesystem read in one adapter cannot
        // freeze the monitor and starve the other adapters' state updates.
        use std::sync::mpsc;
        use std::time::{Duration, Instant};

        let enabled = self.enabled.lock().clone();
        let mut receivers: Vec<(String, mpsc::Receiver<Vec<ActiveThread>>)> = vec![];
        for a in &self.adapters {
            if let Some(false) = enabled.get(a.id()) {
                continue;
            }
            let adapter = a.clone();
            let (tx, rx) = mpsc::channel();
            std::thread::spawn(move || {
                let _ = tx.send(adapter.poll());
            });
            receivers.push((a.id().to_string(), rx));
        }

        let deadline = Instant::now() + Duration::from_secs(5);
        let mut all = Vec::new();
        for (_id, rx) in receivers {
            let remaining = deadline.saturating_duration_since(Instant::now());
            match rx.recv_timeout(remaining) {
                Ok(threads) => all.extend(threads),
                Err(_) => {
                    // Either timed out or the worker thread panicked. Drop
                    // results for this adapter on this poll; we'll try again
                    // next interval. The orphaned thread will finish on its
                    // own and its result will be discarded by the dropped rx.
                }
            }
        }
        (self.callback)(all);
    }
}
