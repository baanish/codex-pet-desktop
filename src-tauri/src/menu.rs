use crate::types::{ActiveThread, ThreadStatus};
use crate::AppState;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

pub fn handle_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    handle_menu_event(app, id);
}

fn render_config(state: &State<AppState>) -> crate::types::AppConfig {
    let mut cfg = state.config.get();
    let (ox, oy) = *state.overlay_origin.lock();
    cfg.position.x -= ox as f64;
    cfg.position.y -= oy as f64;
    cfg
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

const SPEED_CHOICES: &[f64] = &[0.25, 0.33, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0];
const TEXT_SIZES: &[(&str, &str)] = &[
    ("Small", "small"),
    ("Medium", "medium"),
    ("Large", "large"),
    ("Extra Large", "xlarge"),
];
const SCALE_CHOICES: &[(&str, f64)] = &[
    ("25%", 0.25),
    ("33%", 0.33),
    ("50%", 0.5),
    ("75%", 0.75),
    ("100%", 1.0),
    ("150%", 1.5),
    ("200%", 2.0),
    ("300%", 3.0),
];
const POLL_CHOICES: &[(&str, u64)] = &[
    ("10s", 10_000),
    ("30s", 30_000),
    ("60s", 60_000),
    ("120s", 120_000),
];

pub fn popup<R: Runtime>(app: &AppHandle<R>) {
    let state: State<AppState> = match app.try_state::<AppState>() {
        Some(s) => s,
        None => return,
    };
    let cfg = state.config.get();
    let agents = state
        .monitor
        .lock()
        .as_ref()
        .map(|m| m.list_adapters())
        .unwrap_or_default();
    let dismissible_thread = state
        .monitor
        .lock()
        .as_ref()
        .and_then(|m| top_dismissible_thread(m.visible_threads()));
    let pets = crate::pet_loader::load_pets();
    let debug_anim = state.debug_animation.lock().clone();
    let pingpong = state.pingpong.lock().clone();

    let menu = match build_menu(
        app,
        &cfg,
        &agents,
        &pets,
        debug_anim.as_deref(),
        &pingpong,
        dismissible_thread.as_ref(),
    ) {
        Ok(m) => m,
        Err(_) => return,
    };
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.popup_menu(&menu);
    }
}

fn build_menu<R: Runtime>(
    app: &AppHandle<R>,
    cfg: &crate::types::AppConfig,
    agents: &[(String, String)],
    pets: &[crate::types::PetInfo],
    debug_anim: Option<&str>,
    pingpong: &std::collections::HashMap<String, bool>,
    dismissible_thread: Option<&ActiveThread>,
) -> tauri::Result<Menu<R>> {
    let menu = Menu::new(app)?;

    // ---------- Pet selector ----------
    let pet_sub = Submenu::new(app, "Pet", true)?;
    if pets.is_empty() {
        let item = MenuItem::with_id(
            app,
            "pet:none",
            "No pets in ~/.codex/pets",
            false,
            None::<&str>,
        )?;
        pet_sub.append(&item)?;
    } else {
        let selected_id = cfg.selected_pet_id.as_deref().unwrap_or_else(|| {
            pets.first().map(|p| p.id.as_str()).unwrap_or("")
        });
        for pet in pets {
            let item = CheckMenuItem::with_id(
                app,
                format!("pet:{}", pet.id),
                &pet.display_name,
                true,
                pet.id == selected_id,
                None::<&str>,
            )?;
            pet_sub.append(&item)?;
        }
    }
    menu.append(&pet_sub)?;

    // ---------- Size submenu ----------
    let size_sub = Submenu::new(app, "Size", true)?;
    for (label, value) in SCALE_CHOICES {
        let item = CheckMenuItem::with_id(
            app,
            format!("scale:{}", value),
            *label,
            true,
            (cfg.scale - value).abs() < 1e-6,
            None::<&str>,
        )?;
        size_sub.append(&item)?;
    }
    menu.append(&size_sub)?;

    // ---------- Text Size submenu ----------
    let text_sub = Submenu::new(app, "Text Size", true)?;
    for (label, value) in TEXT_SIZES {
        let item = CheckMenuItem::with_id(
            app,
            format!("text:{}", value),
            *label,
            true,
            cfg.text_size == *value,
            None::<&str>,
        )?;
        text_sub.append(&item)?;
    }
    menu.append(&text_sub)?;

    // ---------- Poll Interval ----------
    let poll_sub = Submenu::new(app, "Poll Interval", true)?;
    for (label, value) in POLL_CHOICES {
        let item = CheckMenuItem::with_id(
            app,
            format!("poll:{}", value),
            *label,
            true,
            cfg.poll_interval_ms == *value,
            None::<&str>,
        )?;
        poll_sub.append(&item)?;
    }
    menu.append(&poll_sub)?;

    // ---------- Agents ----------
    let agents_sub = Submenu::new(app, "Agents", true)?;
    if agents.is_empty() {
        let item = MenuItem::with_id(app, "agents:none", "No agents installed", false, None::<&str>)?;
        agents_sub.append(&item)?;
    } else {
        for (id, display) in agents {
            let enabled = cfg.enabled_agents.get(id).copied().unwrap_or(true);
            let item = CheckMenuItem::with_id(
                app,
                format!("agent:{}", id),
                display,
                true,
                enabled,
                None::<&str>,
            )?;
            agents_sub.append(&item)?;
        }
    }
    menu.append(&agents_sub)?;

    menu.append(&PredefinedMenuItem::separator(app)?)?;

    // ---------- Animation Speed ----------
    let speed_sub = Submenu::new(app, "Animation Speed", true)?;
    for name in ALL_ANIMATIONS {
        let inner = Submenu::new(app, *name, true)?;
        let current = cfg.animation_speeds.get(*name).copied().unwrap_or(1.0);
        for s in SPEED_CHOICES {
            let item = CheckMenuItem::with_id(
                app,
                format!("speed:{}:{}", name, s),
                format!("{}×", s),
                true,
                (current - s).abs() < 1e-6,
                None::<&str>,
            )?;
            inner.append(&item)?;
        }
        speed_sub.append(&inner)?;
    }
    menu.append(&speed_sub)?;

    // ---------- Ping-pong ----------
    let pingpong_sub = Submenu::new(app, "Ping-pong", true)?;
    for name in ALL_ANIMATIONS {
        let enabled = pingpong.get(*name).copied().unwrap_or_else(|| default_pingpong(name));
        let item = CheckMenuItem::with_id(
            app,
            format!("pingpong:{}", name),
            *name,
            true,
            enabled,
            None::<&str>,
        )?;
        pingpong_sub.append(&item)?;
    }
    menu.append(&pingpong_sub)?;

    // ---------- Debug · Animation ----------
    let debug_sub = Submenu::new(app, "Debug · Animation", true)?;
    let auto = CheckMenuItem::with_id(
        app,
        "debuganim:__auto__",
        "Auto (thread-driven)",
        true,
        debug_anim.is_none(),
        None::<&str>,
    )?;
    debug_sub.append(&auto)?;
    debug_sub.append(&PredefinedMenuItem::separator(app)?)?;
    for name in ALL_ANIMATIONS {
        let item = CheckMenuItem::with_id(
            app,
            format!("debuganim:{}", name),
            *name,
            true,
            debug_anim == Some(*name),
            None::<&str>,
        )?;
        debug_sub.append(&item)?;
    }
    menu.append(&debug_sub)?;

    menu.append(&PredefinedMenuItem::separator(app)?)?;

    let hide_app_server = CheckMenuItem::with_id(
        app,
        "hide-codex-app-server",
        "Hide codex app-server sessions",
        true,
        cfg.hide_codex_app_server,
        None::<&str>,
    )?;
    menu.append(&hide_app_server)?;

    let dismiss_label = dismissible_thread
        .and_then(|t| t.title.as_deref())
        .map(|title| format!("Dismiss Thread: {}", menu_title(title)))
        .unwrap_or_else(|| "Dismiss Thread".into());
    let dismiss = MenuItem::with_id(
        app,
        "dismiss-thread",
        dismiss_label,
        dismissible_thread.is_some(),
        None::<&str>,
    )?;
    menu.append(&dismiss)?;

    let aot = CheckMenuItem::with_id(
        app,
        "always-on-top",
        "Always on Top",
        true,
        cfg.always_on_top,
        None::<&str>,
    )?;
    menu.append(&aot)?;

    menu.append(&PredefinedMenuItem::separator(app)?)?;

    let quit = MenuItem::with_id(app, "quit", "Quit", true, Some("CmdOrCtrl+Q"))?;
    menu.append(&quit)?;

    // Menu events bubble to the App via Builder::on_menu_event in lib.rs.
    Ok(menu)
}

fn top_dismissible_thread(mut threads: Vec<ActiveThread>) -> Option<ActiveThread> {
    threads.retain(|t| t.status != ThreadStatus::Idle && t.status != ThreadStatus::Busy);
    threads.sort_by_key(|t| std::cmp::Reverse(thread_priority(t.status)));
    threads.into_iter().next()
}

fn thread_priority(status: ThreadStatus) -> u8 {
    match status {
        ThreadStatus::Error => 5,
        ThreadStatus::Busy => 4,
        ThreadStatus::Waiting => 3,
        ThreadStatus::Open => 2,
        ThreadStatus::Stale => 1,
        ThreadStatus::Idle => 0,
    }
}

fn menu_title(title: &str) -> String {
    let title = title.trim().trim_start_matches('#').trim();
    let mut chars = title.chars();
    let short: String = chars.by_ref().take(32).collect();
    if chars.next().is_some() {
        format!("{}...", short)
    } else if short.is_empty() {
        "Untitled thread".into()
    } else {
        short
    }
}

fn default_pingpong(name: &str) -> bool {
    matches!(
        name,
        "idle" | "running-right" | "running-left" | "waiting" | "review"
    )
}

fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };

    if let Some(value) = id.strip_prefix("scale:") {
        if let Ok(v) = value.parse::<f64>() {
            state.config.update(|c| c.scale = v);
            let cfg = render_config(&state);
            let _ = app.emit("config", &cfg);
        }
    } else if let Some(value) = id.strip_prefix("text:") {
        state.config.update(|c| c.text_size = value.into());
        let cfg = render_config(&state);
        let _ = app.emit("config", &cfg);
    } else if let Some(value) = id.strip_prefix("poll:") {
        if let Ok(v) = value.parse::<u64>() {
            state.config.update(|c| c.poll_interval_ms = v);
            if let Some(monitor) = state.monitor.lock().as_ref() {
                monitor.set_interval(v);
            }
        }
    } else if let Some(rest) = id.strip_prefix("speed:") {
        if let Some((name, val)) = rest.split_once(':') {
            if let Ok(speed) = val.parse::<f64>() {
                state.config.update(|c| {
                    c.animation_speeds.insert(name.into(), speed);
                });
                let _ = app.emit(
                    "animation-speed",
                    serde_json::json!({ "name": name, "speed": speed }),
                );
            }
        }
    } else if let Some(name) = id.strip_prefix("agent:") {
        let cfg = state.config.update(|c| {
            let cur = c.enabled_agents.get(name).copied().unwrap_or(true);
            c.enabled_agents.insert(name.into(), !cur);
        });
        if let Some(monitor) = state.monitor.lock().as_ref() {
            monitor.set_enabled(cfg.enabled_agents.clone());
            monitor.trigger();
        }
    } else if let Some(name) = id.strip_prefix("pingpong:") {
        let mut g = state.pingpong.lock();
        let cur = g
            .get(name)
            .copied()
            .unwrap_or_else(|| default_pingpong(name));
        let enabled = !cur;
        g.insert(name.into(), enabled);
        drop(g);
        let _ = app.emit(
            "pingpong-override",
            serde_json::json!({ "name": name, "enabled": enabled }),
        );
    } else if let Some(name) = id.strip_prefix("debuganim:") {
        let value = if name == "__auto__" {
            None
        } else {
            Some(name.to_string())
        };
        *state.debug_animation.lock() = value.clone();
        let _ = app.emit("debug-animation", &value);
    } else if let Some(pet_id) = id.strip_prefix("pet:") {
        if pet_id == "none" {
            return;
        }
        state.config.update(|c| {
            c.selected_pet_id = Some(pet_id.into());
        });
        let pets = crate::pet_loader::load_pets();
        if let Some(pet) = pets.into_iter().find(|p| p.id == pet_id) {
            let _ = app.emit("pet-data", &pet);
        }
    } else if id == "always-on-top" {
        let cfg = state.config.update(|c| c.always_on_top = !c.always_on_top);
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.set_always_on_top(cfg.always_on_top);
        }
    } else if id == "hide-codex-app-server" {
        let cfg = state.config.update(|c| {
            c.hide_codex_app_server = !c.hide_codex_app_server;
        });
        // Live-mutate the shared switch the codex adapter reads on every
        // poll. Re-poll right away so the UI reflects the change without
        // waiting for the next interval.
        *state.hide_codex_app_server.lock() = cfg.hide_codex_app_server;
        if let Some(monitor) = state.monitor.lock().as_ref() {
            monitor.trigger();
        }
    } else if id == "dismiss-thread" {
        if let Some(monitor) = state.monitor.lock().as_ref() {
            if let Some(thread) = top_dismissible_thread(monitor.visible_threads()) {
                monitor.dismiss_thread(thread);
            }
        }
    } else if id == "quit" {
        app.exit(0);
    }
}
