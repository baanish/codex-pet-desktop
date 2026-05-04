mod asar;
mod config;
mod menu;
mod pet_loader;
mod threads;
mod types;
mod window;

use std::collections::HashMap;
use std::sync::Arc;

use config::ConfigStore;
use parking_lot::Mutex;
use tauri::{AppHandle, Emitter, Manager, RunEvent};
use threads::adapter::ThreadAdapter;
use threads::{
    claude_code::ClaudeCodeAdapter, codex::CodexAdapter, opencode::OpenCodeAdapter, ThreadMonitor,
};
use types::{ActiveThread, AppConfig, Position};

pub struct AppState {
    pub config: ConfigStore,
    pub monitor: Mutex<Option<Arc<ThreadMonitor>>>,
    pub pet_bounds: Mutex<Option<PetBounds>>,
    pub is_dragging: Mutex<bool>,
    pub debug_animation: Mutex<Option<String>>,
    pub pingpong: Mutex<HashMap<String, bool>>,
}

#[derive(Debug, Clone, Copy)]
pub struct PetBounds {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

fn shared_user_data_dir() -> Option<std::path::PathBuf> {
    let home = dirs::home_dir()?;
    #[cfg(target_os = "macos")]
    {
        return Some(home.join("Library/Application Support/codex-pet-desktop"));
    }
    #[cfg(target_os = "linux")]
    {
        return Some(home.join(".config/codex-pet-desktop"));
    }
    #[cfg(target_os = "windows")]
    {
        return Some(home.join("AppData/Roaming/codex-pet-desktop"));
    }
    #[allow(unreachable_code)]
    None
}

const ALL_ANIMATIONS: &[&str] = &[
    "idle",
    "running-right",
    "running-left",
    "waving",
    "jumping",
    "failed",
    "waiting",
    "running",
    "review",
];

const DEFAULT_PINGPONG: &[(&str, bool)] = &[
    ("idle", true),
    ("running-right", true),
    ("running-left", true),
    ("waving", false),
    ("jumping", false),
    ("failed", false),
    ("waiting", true),
    ("running", false),
    ("review", true),
];

#[tauri::command]
fn get_config(state: tauri::State<AppState>) -> AppConfig {
    state.config.get()
}

#[tauri::command]
fn save_position(state: tauri::State<AppState>, position: Position) {
    state.config.update(|c| c.position = position);
}

#[tauri::command]
fn save_scale(state: tauri::State<AppState>, scale: f64) {
    if scale.is_finite() && scale > 0.0 {
        state.config.update(|c| c.scale = scale);
    }
}

#[tauri::command]
fn trigger_poll(state: tauri::State<AppState>) {
    if let Some(monitor) = state.monitor.lock().as_ref() {
        monitor.trigger();
    }
}

#[tauri::command]
fn set_ignore_mouse_events(window: tauri::WebviewWindow, ignore: bool) {
    let _ = window.set_ignore_cursor_events(ignore);
}

#[tauri::command]
fn set_pet_bounds(state: tauri::State<AppState>, x: f64, y: f64, w: f64, h: f64) {
    *state.pet_bounds.lock() = Some(PetBounds { x, y, w, h });
}

#[tauri::command]
fn set_dragging(state: tauri::State<AppState>, dragging: bool) {
    *state.is_dragging.lock() = dragging;
}

#[tauri::command]
fn show_context_menu(app: AppHandle) {
    menu::popup(&app);
}

#[tauri::command]
fn list_pets() -> Vec<types::PetInfo> {
    pet_loader::load_pets()
}

#[tauri::command]
fn read_pet_image(pet_id: String) -> Result<Vec<u8>, String> {
    // ID-based lookup, not path-based. The renderer is untrusted enough that
    // accepting an arbitrary filesystem path here would make any renderer
    // injection a "read any file the user can read" primitive. By going
    // through pet_loader the only readable bytes are spritesheets in the
    // known pet directories (~/.codex/pets/, the built-in cache, or the
    // CODEX_PETS_DIR override).
    let pets = pet_loader::load_pets();
    let Some(pet) = pets.iter().find(|p| p.id == pet_id) else {
        return Err(format!("unknown pet: {}", pet_id));
    };
    std::fs::read(&pet.spritesheet_abs_path)
        .map_err(|e| format!("read pet image {}: {}", pet_id, e))
}

pub fn run() {
    #[cfg(target_os = "macos")]
    {
        // Hide from Dock + cmd-tab; bypasses macOS Stage Manager window
        // management at the same time.
        // Activation policy is set on the AppHandle once the app is ready.
    }

    tauri::Builder::default()
        .on_menu_event(|app, event| {
            menu::handle_event(app, event.id().0.as_str());
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_position,
            save_scale,
            trigger_poll,
            set_ignore_mouse_events,
            show_context_menu,
            set_text_size,
            set_agent_enabled,
            set_animation_speed,
            set_pingpong,
            set_debug_animation,
            set_always_on_top,
            request_quit,
            list_pets,
            read_pet_image,
            set_pet_bounds,
            set_dragging,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                use tauri::ActivationPolicy;
                let _ = app.set_activation_policy(ActivationPolicy::Accessory);
            }

            // Use the same config path as the Electron build so settings
            // persist when the user switches between the two implementations.
            let user_data = shared_user_data_dir().unwrap_or_else(|| {
                app.path().app_data_dir().expect("no app_data_dir")
            });
            std::fs::create_dir_all(&user_data).ok();
            let config = ConfigStore::new(user_data);

            let win = window::create_pet_window(app.handle())?;

            // Apply restored position once the window settles
            let cfg_initial = config.get();
            let _ = win.set_always_on_top(cfg_initial.always_on_top);

            // Build monitor wired to emit thread-state events
            let app_handle = app.handle().clone();
            let adapters: Vec<Arc<dyn ThreadAdapter>> = vec![
                Arc::new(OpenCodeAdapter::new()),
                Arc::new(ClaudeCodeAdapter::new()),
                Arc::new(CodexAdapter::new()),
            ];
            let monitor = Arc::new(ThreadMonitor::new(adapters, move |threads: Vec<ActiveThread>| {
                let _ = app_handle.emit("thread-state", &threads);
            }));
            monitor.set_enabled(cfg_initial.enabled_agents.clone());
            monitor.set_interval(cfg_initial.poll_interval_ms);
            monitor.start();

            let pingpong: HashMap<String, bool> = DEFAULT_PINGPONG
                .iter()
                .map(|(k, v)| (k.to_string(), *v))
                .collect();

            let state = AppState {
                config,
                monitor: Mutex::new(Some(monitor)),
                pet_bounds: Mutex::new(None),
                is_dragging: Mutex::new(false),
                debug_animation: Mutex::new(None),
                pingpong: Mutex::new(pingpong),
            };
            app.manage(state);

            // Cursor-position polling: Tauri has no `forward: true` for
            // ignore_cursor_events, so we sample the global cursor and toggle
            // click-through based on whether it's over the pet's CSS bounds.
            let app_for_thread = app.handle().clone();
            std::thread::spawn(move || cursor_polling_loop(app_for_thread));

            // Push pet data + config to renderer
            let pets = pet_loader::load_pets();
            if let Some(selected) = pets
                .iter()
                .find(|p| Some(&p.id) == cfg_initial.selected_pet_id.as_ref())
                .or_else(|| pets.first())
                .cloned()
            {
                let _ = app.emit("pet-data", &selected);
            }
            let _ = app.emit("config", &cfg_initial);
            for (name, speed) in &cfg_initial.animation_speeds {
                let _ = app.emit(
                    "animation-speed",
                    serde_json::json!({ "name": name, "speed": speed }),
                );
            }
            for (name, enabled) in DEFAULT_PINGPONG {
                let _ = app.emit(
                    "pingpong-override",
                    serde_json::json!({ "name": name, "enabled": enabled }),
                );
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            if let RunEvent::ExitRequested { .. } = event {
                // graceful shutdown handled by drop
            }
        });
}

// ---------- additional commands invoked from menu / renderer ----------

#[tauri::command]
fn set_text_size(state: tauri::State<AppState>, app: AppHandle, size: String) {
    let cfg = state.config.update(|c| c.text_size = size.clone());
    let _ = app.emit("config", &cfg);
}

#[tauri::command]
fn set_agent_enabled(state: tauri::State<AppState>, id: String, enabled: bool) {
    let cfg = state.config.update(|c| {
        c.enabled_agents.insert(id.clone(), enabled);
    });
    if let Some(monitor) = state.monitor.lock().as_ref() {
        monitor.set_enabled(cfg.enabled_agents.clone());
        monitor.trigger();
    }
}

#[tauri::command]
fn set_animation_speed(
    state: tauri::State<AppState>,
    app: AppHandle,
    name: String,
    speed: f64,
) {
    state.config.update(|c| {
        c.animation_speeds.insert(name.clone(), speed);
    });
    let _ = app.emit(
        "animation-speed",
        serde_json::json!({ "name": name, "speed": speed }),
    );
}

#[tauri::command]
fn set_pingpong(state: tauri::State<AppState>, app: AppHandle, name: String, enabled: bool) {
    state.pingpong.lock().insert(name.clone(), enabled);
    let _ = app.emit(
        "pingpong-override",
        serde_json::json!({ "name": name, "enabled": enabled }),
    );
}

#[tauri::command]
fn set_debug_animation(state: tauri::State<AppState>, app: AppHandle, name: Option<String>) {
    *state.debug_animation.lock() = name.clone();
    let _ = app.emit("debug-animation", &name);
}

#[tauri::command]
fn set_always_on_top(state: tauri::State<AppState>, window: tauri::WebviewWindow, enabled: bool) {
    state.config.update(|c| c.always_on_top = enabled);
    let _ = window.set_always_on_top(enabled);
}

#[tauri::command]
fn request_quit(app: AppHandle) {
    app.exit(0);
}

fn cursor_polling_loop(app: AppHandle) {
    use std::time::Duration;
    let mut last_state: Option<bool> = None;
    loop {
        // 8ms ≈ 120Hz: keeps the race between cursor-arriving-on-pet and a
        // user click below human-detectable latency. The thread is otherwise
        // doing essentially nothing, so this is cheap.
        std::thread::sleep(Duration::from_millis(8));
        let Some(window) = app.get_webview_window("main") else {
            continue;
        };
        let Some(state) = app.try_state::<AppState>() else { continue };

        if *state.is_dragging.lock() {
            // While the user is dragging, keep events flowing.
            if last_state != Some(false) {
                let _ = window.set_ignore_cursor_events(false);
                last_state = Some(false);
            }
            continue;
        }

        let cursor = match app.cursor_position() {
            Ok(c) => c,
            Err(_) => continue,
        };
        let scale = window.scale_factor().unwrap_or(1.0);
        let inner = match window.inner_position() {
            Ok(p) => p,
            Err(_) => continue,
        };

        let cursor_css_x = (cursor.x - inner.x as f64) / scale;
        let cursor_css_y = (cursor.y - inner.y as f64) / scale;

        let bounds = *state.pet_bounds.lock();
        let in_pet = match bounds {
            Some(b) => {
                let pad = 4.0;
                cursor_css_x >= b.x - pad
                    && cursor_css_x < b.x + b.w + pad
                    && cursor_css_y >= b.y - pad
                    && cursor_css_y < b.y + b.h + pad
            }
            None => false,
        };

        let want_ignore = !in_pet;
        if last_state != Some(want_ignore) {
            let _ = window.set_ignore_cursor_events(want_ignore);
            last_state = Some(want_ignore);
        }
    }
}

#[allow(dead_code)]
fn pingpong_defaults_map() -> HashMap<String, bool> {
    DEFAULT_PINGPONG
        .iter()
        .map(|(k, v)| (k.to_string(), *v))
        .collect()
}

#[allow(dead_code)]
const _ANIMATIONS_USED: &[&str] = ALL_ANIMATIONS;
