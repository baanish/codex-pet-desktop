use crate::types::AppConfig;
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::Arc;

pub struct ConfigStore {
    path: PathBuf,
    state: Arc<Mutex<AppConfig>>,
}

impl ConfigStore {
    pub fn new(user_data_dir: PathBuf) -> Self {
        let path = user_data_dir.join("config.json");
        let state = Arc::new(Mutex::new(load(&path)));
        Self { path, state }
    }

    pub fn get(&self) -> AppConfig {
        self.state.lock().clone()
    }

    pub fn set(&self, cfg: AppConfig) {
        *self.state.lock() = cfg;
        self.flush();
    }

    pub fn update<F: FnOnce(&mut AppConfig)>(&self, f: F) -> AppConfig {
        let mut g = self.state.lock();
        f(&mut g);
        let cfg = g.clone();
        drop(g);
        self.flush();
        cfg
    }

    fn flush(&self) {
        let cfg = self.state.lock().clone();
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&cfg) {
            let _ = std::fs::write(&self.path, json);
        }
    }
}

fn load(path: &PathBuf) -> AppConfig {
    if let Ok(s) = std::fs::read_to_string(path) {
        if let Ok(cfg) = serde_json::from_str::<AppConfig>(&s) {
            return cfg;
        }
        // Try merging with defaults if shape changed
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&s) {
            let mut def = serde_json::to_value(AppConfig::default()).unwrap();
            merge_values(&mut def, &value);
            if let Ok(cfg) = serde_json::from_value::<AppConfig>(def) {
                return cfg;
            }
        }
    }
    AppConfig::default()
}

fn merge_values(dst: &mut serde_json::Value, src: &serde_json::Value) {
    if let (Some(d), Some(s)) = (dst.as_object_mut(), src.as_object()) {
        for (k, v) in s {
            match d.get_mut(k) {
                Some(existing) => merge_values(existing, v),
                None => {
                    d.insert(k.clone(), v.clone());
                }
            }
        }
    } else {
        *dst = src.clone();
    }
}
