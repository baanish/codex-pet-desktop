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
        let mut cfg = load(&path);
        normalize(&mut cfg);
        let state = Arc::new(Mutex::new(cfg));
        Self { path, state }
    }

    pub fn get(&self) -> AppConfig {
        self.state.lock().clone()
    }

    pub fn set(&self, cfg: AppConfig) {
        let mut cfg = cfg;
        normalize(&mut cfg);
        *self.state.lock() = cfg;
        self.flush();
    }

    pub fn update<F: FnOnce(&mut AppConfig)>(&self, f: F) -> AppConfig {
        let mut g = self.state.lock();
        f(&mut g);
        normalize(&mut g);
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

/// Clamp / sanitize values that the rest of the app trusts. The persisted
/// JSON is editable by the user (and shared with the legacy Electron build,
/// which had its own constraints), so we treat anything we read off disk as
/// untrusted: a `pollIntervalMs: 0` would otherwise spin the monitor at 100%
/// CPU, a non-finite scale would NaN-poison the renderer, etc.
fn normalize(cfg: &mut AppConfig) {
    if !cfg.scale.is_finite() || cfg.scale <= 0.0 {
        cfg.scale = 1.0;
    }
    cfg.scale = cfg.scale.clamp(0.1, 10.0);

    // Floor at 1s so the polling thread can't tight-loop. 10 minutes is the
    // documented upper end (the menu offers 10s..120s, but a hand-edited
    // value somewhere in that range is fine).
    cfg.poll_interval_ms = cfg.poll_interval_ms.clamp(1_000, 600_000);

    let allowed_text = ["small", "medium", "large", "xlarge"];
    if !allowed_text.contains(&cfg.text_size.as_str()) {
        cfg.text_size = "medium".into();
    }

    if !cfg.position.x.is_finite() {
        cfg.position.x = 0.0;
    }
    if !cfg.position.y.is_finite() {
        cfg.position.y = 0.0;
    }

    for (_, v) in cfg.animation_speeds.iter_mut() {
        if !v.is_finite() || *v <= 0.0 {
            *v = 1.0;
        } else {
            *v = v.clamp(0.1, 10.0);
        }
    }

    if let Some(id) = cfg.selected_pet_id.as_deref() {
        // Same allowlist pet_loader uses; reject ids that could be coerced
        // into a path traversal further down the line.
        let safe = !id.is_empty()
            && id.len() <= 64
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        if !safe {
            cfg.selected_pet_id = None;
        }
    }
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
