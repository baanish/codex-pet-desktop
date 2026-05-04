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
        let enabled = self.enabled.lock().clone();
        let mut all = Vec::new();
        for a in &self.adapters {
            if let Some(false) = enabled.get(a.id()) {
                continue;
            }
            let mut threads = a.poll();
            all.append(&mut threads);
        }
        (self.callback)(all);
    }
}
