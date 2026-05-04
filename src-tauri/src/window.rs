use tauri::{
    AppHandle, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

pub fn create_pet_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    // Compute the bounding box of every connected monitor. The overlay needs
    // to span the full virtual desktop so the user can drop the pet on any
    // display, not just the primary one.
    let monitors = app.available_monitors().unwrap_or_default();
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    for m in &monitors {
        let pos = m.position();
        let size = m.size();
        min_x = min_x.min(pos.x);
        min_y = min_y.min(pos.y);
        max_x = max_x.max(pos.x + size.width as i32);
        max_y = max_y.max(pos.y + size.height as i32);
    }
    if monitors.is_empty() {
        // No monitors enumerated yet (rare); fall back to primary so we still
        // get *something* on screen.
        let primary = app
            .primary_monitor()?
            .expect("no monitor available");
        let pos = primary.position();
        let size = primary.size();
        min_x = pos.x;
        min_y = pos.y;
        max_x = pos.x + size.width as i32;
        max_y = pos.y + size.height as i32;
    }

    let oversized_w = (max_x - min_x) + 200;
    let oversized_h = (max_y - min_y) + 200;
    let target_x = min_x - 100;
    let target_y = min_y - 100;

    // DEV MODE: smaller decorated window so we can see whether the renderer
    // loaded. Will be turned back into a transparent overlay once the JS side
    // proves it's painting.
    let dev_mode = std::env::var("PET_DEV_VISIBLE").is_ok();

    let mut builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("Codex Pet")
        .always_on_top(true)
        .skip_taskbar(true)
        .visible_on_all_workspaces(true);

    if dev_mode {
        builder = builder.initialization_script("window.__PET_DEBUG__ = true;");
    }

    if dev_mode {
        builder = builder
            .inner_size(1400.0, 900.0)
            .position(100.0, 60.0)
            .decorations(true)
            .resizable(true)
            .transparent(false);
    } else {
        builder = builder
            .inner_size(oversized_w as f64, oversized_h as f64)
            .position(target_x as f64, target_y as f64)
            .transparent(true)
            .decorations(false)
            .shadow(false)
            .resizable(false)
            .minimizable(false)
            .maximizable(false);
    }

    let win = builder.build()?;

    // The macOS Stage Manager x-clamp only applies at construction. Re-set
    // bounds afterwards to bypass the clamp and cover the strip area.
    if !dev_mode {
        let _ = win.set_size(PhysicalSize::new(oversized_w as u32, oversized_h as u32));
        let _ = win.set_position(PhysicalPosition::new(target_x, target_y));
    }

    // Initial state: ignore cursor everywhere. The cursor-polling thread in
    // lib.rs flips this to false the moment the cursor hovers over the pet.
    if !dev_mode {
        let _ = win.set_ignore_cursor_events(true);
    }

    Ok(win)
}
