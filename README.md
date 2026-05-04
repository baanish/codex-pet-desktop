# Codex Pet Desktop

A pixel-art pet that lives on your desktop and reacts to whichever AI coding agent
is currently working — Codex, Claude Code, or opencode. The pet wakes up when
your agent is thinking, naps when it's idle, sulks when it errors, and labels
the active thread by name in a small card you can pin alongside it.

The pitch: **Codex Pets, without sacrificing gigabytes of RAM to the Codex app.
Cross-platform and cross-agent while we were at it.**

|                              | Original (Electron) | This (Tauri) |
| ---                          | ---                 | ---          |
| App bundle on disk           | ~720 MB             | **4.5 MB**   |
| Idle resident memory         | ~380 MB             | **~100 MB**  |
| Idle CPU                     | ~10%                | **~1%**      |
| Targets                      | macOS               | **macOS, Windows, Linux** |

## Screenshots

The pet renders as a transparent always-on-top overlay. A thread card appears
above it whenever an agent is doing real work; a `+N` chevron next to the pet
expands the card to show every active thread. Hover the pet to interact, drag
it anywhere on screen, right-click for the menu.

## Supported agents

| Agent          | Detection                                       |
| ---            | ---                                             |
| **Codex CLI**  | `~/.codex/tmp/arg0/codex-*/.lock` + `~/.codex/state_5.sqlite` |
| **Claude Code**| `~/.claude/sessions/*.json` (with live PIDs)    |
| **opencode**   | `~/.local/share/opencode/opencode.db` + WAL liveness |

Agents that aren't installed are silently skipped. You can disable any of them
at runtime from the right-click menu.

## Install

### Pre-built (macOS)

Download the latest `Codex Pet Desktop_*.dmg` release, drag the app into
`/Applications`, then on first launch right-click → Open (the build is
unsigned).

### From source

You need [Rust](https://rustup.rs/) and Node 18+. Then:

```bash
git clone <repo>
cd codex-pet-desktop
npm install
npm run tauri:build      # produces a bundle for the host platform
```

`tauri build` packages whatever targets your host can produce:
`.app` + `.dmg` on macOS, `.deb` + `.AppImage` + `.rpm` on Linux,
`.msi` + NSIS `.exe` on Windows. Only macOS has been smoke-tested by
the maintainer; Linux/Windows builds compile and bundle but you may
hit platform-specific edges. PRs welcome for verification on those
platforms.

For local development:

```bash
npm run dev              # tauri dev — auto-reloads Rust + frontend
```

To run the existing release binary directly (after `tauri:build`):

```bash
./src-tauri/target/release/codex-pet-desktop
```

## Pets

Pets live in `~/.codex/pets/<id>/` and consist of two files:

```
~/.codex/pets/orion/
├── pet.json
└── spritesheet.webp
```

`pet.json` shape:

```json
{
  "id": "orion",
  "displayName": "Orion",
  "description": "A tiny orange astronaut cat companion.",
  "spritesheetPath": "spritesheet.webp"
}
```

The spritesheet is an 8-column × 9-row grid of 192×208 cells. Each row is one
animation; the columns are the frames (left→right). See
[`src/renderer/atlas.ts`](src/renderer/atlas.ts) for the row → animation
mapping. To install a pet, drop the directory under `~/.codex/pets/` and pick
it from **right-click → Pet**.

You can override the pet directory location with `CODEX_PETS_DIR=/some/path`.

## Animations

| Row | Name              | Used for                         |
| --- | ---               | ---                              |
| 0   | `idle`            | thread status `idle`/`stale`     |
| 1   | `running-right`   | dragging right                   |
| 2   | `running-left`    | dragging left                    |
| 3   | `waving`          | unused (debug menu)              |
| 4   | `jumping`         | drag start / drag-pause          |
| 5   | `failed`          | thread status `error`            |
| 6   | `waiting`         | thread status `waiting`          |
| 7   | `running`         | thread status `busy`             |
| 8   | `review`          | unused (debug menu)              |

Animations can be ping-ponged (forward → reverse → forward) per-animation from
**Ping-pong** in the menu. Per-animation playback speed lives under
**Animation Speed**.

## Right-click menu

- **Pet** — switch between installed pets.
- **Size** — sprite scale (50% → 300%).
- **Text Size** — card title/subtitle font size.
- **Poll Interval** — how often to re-check agent state (10s–120s).
- **Agents** — enable / disable each adapter individually.
- **Animation Speed** — per-animation playback multiplier (0.5× – 3×).
- **Ping-pong** — toggle per-animation forward / yoyo loop.
- **Debug · Animation** — manually pin the sprite to one animation for
  testing; choose **Auto (thread-driven)** to return to normal behavior.
- **Always on Top** — toggle the screen-saver-level always-on-top flag.
- **Quit** — exit the app.

## Architecture

```
codex-pet-desktop/
├── src/                  Frontend (TypeScript, runs in WKWebView/WebView2/WebKitGTK)
│   ├── renderer/
│   │   ├── main.ts       Entry; drag, IPC wiring, debug log
│   │   ├── sprite-engine.ts   Canvas sprite blit, ping-pong, timer-based loop
│   │   ├── thread-label.ts    Card/chevron rendering
│   │   ├── atlas.ts      Pet animation grid metadata
│   │   └── index.html    Static skeleton served by Tauri
│   └── shared/
│       └── types.ts      Shared shapes (used by both halves at compile time)
└── src-tauri/            Rust backend
    ├── src/
    │   ├── lib.rs        Tauri Builder, IPC commands, cursor polling thread
    │   ├── window.rs     Overlay window setup + Stage Manager bypass
    │   ├── menu.rs       Right-click context menu
    │   ├── config.rs     Persistent JSON config (debounced write)
    │   ├── pet_loader.rs Discovers pets in ~/.codex/pets/
    │   ├── threads/      Per-agent adapters + polling monitor
    │   └── types.rs      Rust mirror of shared/types.ts (camelCase JSON)
    ├── tauri.conf.json   App manifest, window/bundle config
    └── Cargo.toml
```

### Why Tauri

The original implementation was Electron. Three Chromium processes plus a
bundled Chromium engine added up to ~720 MB on disk and ~380 MB of resident
memory for what is, in practice, a sprite + a card + three filesystem polls.
Tauri uses the OS's native webview (WKWebView on macOS, WebView2 on Windows,
WebKitGTK on Linux), so we ship 4.5 MB and use ~100 MB at idle. The frontend
code (sprite engine, drag, card UI) ports nearly verbatim — only the IPC
surface changed.

### Click-through

Tauri exposes `set_ignore_cursor_events(bool)` but, unlike Electron, has no
"forward mouse moves while ignoring" mode. We get the same effect by polling
the global cursor position from a Rust thread (every 33 ms), comparing it
against the pet's CSS bounds (reported by the renderer via
`set_pet_bounds`), and toggling the ignore bit accordingly. While the user
is mid-drag, polling stops and the pet keeps capture.

### Stage Manager bypass (macOS)

On macOS Sequoia/Tahoe, Stage Manager clamps a window's `x` at construction
time so it can't extend over the side strip. The fix is to call
`window.set_position()` *after* construction — the OS only applies the clamp
to the constructor argument. See `src-tauri/src/window.rs`.

## Config

Settings live in `~/Library/Application Support/codex-pet-desktop/config.json`
on macOS, `~/.config/codex-pet-desktop/config.json` on Linux, and
`%APPDATA%/codex-pet-desktop/config.json` on Windows. The same path is
shared with the legacy Electron build so settings carry across.

Schema:

```json
{
  "selectedPetId": "orion",
  "position": { "x": 200.0, "y": 900.0 },
  "scale": 0.4,
  "alwaysOnTop": true,
  "pollIntervalMs": 30000,
  "textSize": "medium",
  "enabledAgents": { "opencode": true, "claude-code": true, "codex": true },
  "animationSpeeds": { "running": 1.5 }
}
```

## Debug mode

Launch with the `PET_DEV_VISIBLE=1` env var to get a 1400×900 decorated
window with devtools open, and an on-screen debug log overlay:

```bash
PET_DEV_VISIBLE=1 cargo run
```

Or visit any window with `?debug=1` in the URL hash to enable the in-page
debug log without going into dev-window mode.

## License

MIT — see [LICENSE](LICENSE) (TODO).
