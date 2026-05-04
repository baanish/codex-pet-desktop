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
    /// Discrete hit regions (sprite, card, chevron) reported by the renderer
    /// in CSS coords relative to the overlay. The cursor poll thread tests
    /// against each rect individually; transparent gaps between elements
    /// stay click-through to whatever's underneath.
    pub pet_regions: Mutex<Vec<PetRegion>>,
    pub is_dragging: Mutex<bool>,
    pub debug_animation: Mutex<Option<String>>,
    pub pingpong: Mutex<HashMap<String, bool>>,
    /// Screen position of the overlay window's top-left corner. Used to
    /// translate between the renderer's CSS coordinate system (relative to
    /// the overlay) and the screen coordinates we persist to disk so the
    /// config stays portable across the legacy Electron build and across
    /// multi-monitor layouts where the overlay origin can be negative.
    pub overlay_origin: Mutex<(i32, i32)>,
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct PetRegion {
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
    config_for_renderer(&state)
}

/// The renderer always sees positions in CSS coordinates relative to the
/// overlay window's content area (since that's what `getBoundingClientRect()`
/// gives it). Disk persists the position in screen coordinates, so settings
/// remain portable across the legacy Electron build and across multi-monitor
/// configurations where the overlay's screen origin is non-zero.
fn config_for_renderer(state: &AppState) -> AppConfig {
    let mut cfg = state.config.get();
    let (ox, oy) = *state.overlay_origin.lock();
    cfg.position.x -= ox as f64;
    cfg.position.y -= oy as f64;
    cfg
}

#[tauri::command]
fn save_position(state: tauri::State<AppState>, position: Position) {
    let (ox, oy) = *state.overlay_origin.lock();
    let screen = Position {
        x: position.x + ox as f64,
        y: position.y + oy as f64,
    };
    state.config.update(|c| c.position = screen);
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
fn set_pet_hit_regions(state: tauri::State<AppState>, regions: Vec<PetRegion>) {
    *state.pet_regions.lock() = regions;
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
    // injection a "read any file the user can read" primitive. We go through
    // pet_loader, then independently revalidate the resolved path against the
    // allowed roots — even if a TOCTOU race let pet_loader return a path that
    // points outside (e.g. via a swapped symlink), this final check refuses
    // to serve bytes.
    let pets = pet_loader::load_pets();
    let Some(pet) = pets.iter().find(|p| p.id == pet_id) else {
        return Err(format!("unknown pet: {}", pet_id));
    };
    let path = std::path::Path::new(&pet.spritesheet_abs_path);
    let canonical = path
        .canonicalize()
        .map_err(|e| format!("canonicalize {}: {}", pet.spritesheet_abs_path, e))?;
    let allowed = pet_loader::allowed_pet_roots();
    if !allowed.iter().any(|root| canonical.starts_with(root)) {
        return Err(format!("pet image outside allowed roots: {}", pet_id));
    }
    if let Ok(meta) = std::fs::symlink_metadata(&canonical) {
        if meta.file_type().is_symlink() {
            return Err(format!("pet image is a symlink: {}", pet_id));
        }
    }
    // Bounded read: refuse to allocate gigabytes for a "spritesheet". A
    // poisoned cache file or a hostile custom pet pointing at a huge regular
    // file under an allowed root would otherwise OOM the app on every
    // startup since selectedPetId is persisted.
    const MAX_SPRITE_BYTES: u64 = 16 * 1024 * 1024;
    let metadata = std::fs::metadata(&canonical)
        .map_err(|e| format!("stat pet image {}: {}", pet_id, e))?;
    if !metadata.is_file() {
        return Err(format!("pet image is not a regular file: {}", pet_id));
    }
    if metadata.len() > MAX_SPRITE_BYTES {
        return Err(format!(
            "pet image too large ({} bytes > {} max): {}",
            metadata.len(),
            MAX_SPRITE_BYTES,
            pet_id
        ));
    }
    std::fs::read(&canonical).map_err(|e| format!("read pet image {}: {}", pet_id, e))
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
            set_pet_hit_regions,
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

            let (win, overlay_origin) = window::create_pet_window(app.handle())?;

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
                pet_regions: Mutex::new(Vec::new()),
                is_dragging: Mutex::new(false),
                debug_animation: Mutex::new(None),
                pingpong: Mutex::new(pingpong),
                overlay_origin: Mutex::new(overlay_origin),
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
            // Translate to renderer (CSS) coordinates before emitting.
            let mut cfg_for_renderer = cfg_initial.clone();
            cfg_for_renderer.position.x -= overlay_origin.0 as f64;
            cfg_for_renderer.position.y -= overlay_origin.1 as f64;
            let _ = app.emit("config", &cfg_for_renderer);
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
    state.config.update(|c| c.text_size = size.clone());
    let _ = app.emit("config", &config_for_renderer(&state));
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

        let regions = state.pet_regions.lock().clone();
        let pad = 4.0;
        let in_pet = regions.iter().any(|r| {
            cursor_css_x >= r.x - pad
                && cursor_css_x < r.x + r.w + pad
                && cursor_css_y >= r.y - pad
                && cursor_css_y < r.y + r.h + pad
        });

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
