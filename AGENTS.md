# AGENTS.md

Notes for AI coding assistants (Claude Code, Cursor, Codex CLI, opencode,
etc.) working in this repo. Humans should read [README.md](README.md) and
[CONTRIBUTING.md](CONTRIBUTING.md) instead.

## What this is

A desktop pet that watches for active sessions of three AI coding agents
(Codex, Claude Code, opencode) and reflects their state — running, waiting,
idle, error — through a sprite + a thread card. Tauri-based: Rust backend,
TypeScript renderer in a system webview.

## Repo shape, fast

- `src/renderer/*.ts` — frontend. `main.ts` is the entry. `sprite-engine.ts`
  drives the canvas. `thread-label.ts` renders the card and chevron.
- `src-tauri/src/*.rs` — backend. `lib.rs` wires Tauri builder + commands.
  `window.rs` creates the overlay window. `menu.rs` builds the right-click
  menu. `threads/*.rs` are per-agent polling adapters.
- `src/shared/types.ts` ↔ `src-tauri/src/types.rs` — keep these in sync. Rust
  side uses `serde(rename_all = "camelCase")`.

## Build commands

```bash
npm install
npm run dev               # Tauri dev (auto-reloads Rust)
npm run build             # Renderer only (esbuild + HTML cachebust)
npm run tauri:build       # Full release: .app + .dmg / installer
```

**Important caveat:** Tauri bakes the renderer's `dist/renderer/` into the Rust
binary at `cargo build` time. If you change only TypeScript and run
`cargo run` directly, your changes will NOT be visible — the binary still has
the old frontend. Either re-run `npm run dev`, or
`touch src-tauri/src/lib.rs && cargo build` to force a rebuild.

The HTML cachebust (`?v=__CACHEBUST__` placeholder replaced at build time) is
there for the same reason — WebKit will cache `bundle.js` between launches if
the URL doesn't change.

## IPC surface

The frontend talks to Rust via `invoke()` (commands) and `listen()` (events).
All commands and events live in `src-tauri/src/lib.rs`.

Commands the renderer can call:

| Command                  | Args                          | Returns               |
| ---                      | ---                           | ---                   |
| `get_config`             | —                             | `AppConfig`           |
| `save_position`          | `{ position: {x, y} }`        | —                     |
| `save_scale`             | `{ scale: f64 }`              | —                     |
| `trigger_poll`           | —                             | —                     |
| `set_ignore_mouse_events`| `{ ignore: bool }`            | —                     |
| `show_context_menu`      | —                             | —                     |
| `set_text_size`          | `{ size: string }`            | —                     |
| `set_agent_enabled`      | `{ id, enabled }`             | —                     |
| `set_animation_speed`    | `{ name, speed }`             | —                     |
| `set_pingpong`           | `{ name, enabled }`           | —                     |
| `set_debug_animation`    | `{ name: string \| null }`    | —                     |
| `set_always_on_top`      | `{ enabled: bool }`           | —                     |
| `request_quit`           | —                             | —                     |
| `list_pets`              | —                             | `PetInfo[]`           |
| `read_pet_image`         | `{ path: string }`            | `Vec<u8>` (raw bytes) |
| `set_pet_bounds`         | `{ x, y, w, h }`              | —                     |
| `set_dragging`           | `{ dragging: bool }`          | —                     |

Events the renderer subscribes to:

| Event                | Payload                              |
| ---                  | ---                                  |
| `thread-state`       | `ActiveThread[]`                     |
| `pet-data`           | `PetInfo`                            |
| `config`             | `AppConfig`                          |
| `debug-animation`    | `string \| null`                     |
| `pingpong-override`  | `{ name, enabled }`                  |
| `animation-speed`    | `{ name, speed }`                    |

## Non-obvious things

- **`asset://localhost/...` URLs misbehaved** for absolute filesystem paths in
  this Tauri scope, so we read pet sprite bytes via `read_pet_image` IPC and
  hand a `Blob` URL to the canvas instead of using `convertFileSrc`.
- **`window.screenY` is unreliable** in this WebKit setup — sometimes returns
  the screen height instead of the window's screen-Y. `clampToVisible` in
  `main.ts` bails out when the values look bogus.
- **Tauri does not expose Electron's `forward: true`** for ignore_cursor_events.
  We approximate it: a 33ms-poll thread in Rust queries `app.cursor_position()`,
  converts to CSS pixels using `window.scale_factor()` and `inner_position()`,
  hit-tests against pet bounds reported by the renderer, and toggles
  `set_ignore_cursor_events`. While `is_dragging` is true, the polling skips and
  the renderer keeps capture.
- **macOS Stage Manager clamps `x` at construction**. The fix is to call
  `window.set_position()` *after* the window is built — see `window.rs`.
- **Setup events fire before listeners attach.** Tauri emits during the Rust
  `setup()` callback, but the renderer's `await listen(...)` is registered
  later. Renderer falls back to `invoke('get_config')` and `invoke('list_pets')`
  after listeners are wired.

## Adding things

- **A new agent adapter** → see CONTRIBUTING.md "Adding an agent adapter".
- **A new menu item** → edit `src-tauri/src/menu.rs`. The menu event handler
  (`handle_menu_event`) routes by the item id prefix (`scale:`, `text:`,
  `agent:`, etc.). Use a unique prefix.
- **A new config field** → add to both `src/shared/types.ts` and
  `src-tauri/src/types.rs`. The Rust `ConfigStore` merges defaults forward, so
  old configs without the new field still parse.
- **A new IPC command** → add a `#[tauri::command]` fn in `lib.rs`, register it
  in the `invoke_handler!` list, and call it from the renderer with
  `invoke('snake_case_name', { ... })`.

## Testing changes

There's no automated test suite. The fast loop is:

1. `npm run tauri:build`
2. `cp -r src-tauri/target/release/bundle/macos/'Codex Pet Desktop.app' /Applications/`
3. `pkill -f 'Codex Pet Desktop'; open '/Applications/Codex Pet Desktop.app'`
4. Watch CPU/RAM with `ps -axo rss,pcpu,command | grep codex-pet-desktop`
5. Open `~/Library/Application Support/codex-pet-desktop/pet-diagnostic.json`
   for window-bounds debugging.

For renderer-only iteration, `npm run dev` rebuilds + relaunches faster.

## Branches

- `main` — current implementation.
- `legacy-electron` — the original Electron version, kept for reference. Don't
  introduce changes there unless you're explicitly asked.

## Style

- Comments explain *why*, not *what*. The why-comments around the Stage Manager
  bypass, the cursor polling, the asset-protocol workaround, and the bake-time
  cachebust are load-bearing — leave them in.
- Don't add error handling for cases that can't happen. The renderer assumes
  the Rust side is alive and the Rust side trusts the renderer's bounds. Don't
  layer defensive `try/catch` around every `invoke`.
- Don't introduce new framework dependencies without a strong reason. Part of
  the value of this rewrite is that the runtime cost is small.
