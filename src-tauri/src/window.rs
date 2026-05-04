use tauri::{
    AppHandle, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

pub fn create_pet_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    // Determine display bounds before creating the window.
    let monitor = app
        .primary_monitor()?
        .or_else(|| app.available_monitors().ok().and_then(|v| v.into_iter().next()))
        .expect("no monitor available");

    let size = monitor.size();
    let position = monitor.position();

    let oversized_w = size.width as i32 + 200;
    let oversized_h = size.height as i32 + 200;
    let target_x = position.x - 100;
    let target_y = position.y - 100;

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
