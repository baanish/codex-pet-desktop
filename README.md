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

You need [Rust](https://rustup.rs/) (1.77+) and Node 18+, plus a
platform-specific C/C++ toolchain.

> **Heads up: only macOS is verified.** The Linux and Windows build
> recipes below compile cleanly and `tauri build` produces packages,
> but the maintainer has not run them end-to-end. Expect some
> platform-specific edges (system tray icon paths, font fallback,
> WebView2 redistributable on older Windows installs). PRs that verify
> these flows are welcome.

#### macOS (verified)

```bash
xcode-select --install                  # if you don't have it yet
git clone <repo>
cd codex-pet-desktop
npm install
npm run tauri:build                     # → src-tauri/target/release/bundle/{macos,dmg}/
```

Outputs `Codex Pet Desktop.app` and `Codex Pet Desktop_*.dmg`. The
build is unsigned; first launch needs a right-click → Open or you can
notarize/sign it yourself by configuring `bundle.macOS.signingIdentity`
in `src-tauri/tauri.conf.json`.

#### Linux (untested)

```bash
# Debian/Ubuntu
sudo apt update
sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget \
                    file libxdo-dev libssl-dev libayatana-appindicator3-dev \
                    librsvg2-dev

# Fedora
sudo dnf install -y webkit2gtk4.1-devel openssl-devel curl wget file \
                    libappindicator-gtk3-devel librsvg2-devel

curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -   # if missing
git clone <repo>
cd codex-pet-desktop
npm install
npm run tauri:build                     # → src-tauri/target/release/bundle/{deb,appimage,rpm}/
```

Likely working but not verified by the maintainer:
- Stage Manager / panel-layer behavior is macOS-specific; on GNOME and
  KDE the always-on-top semantics may differ.
- `~/.codex/pets/` and `~/.config/codex-pet-desktop/` paths are correct
  per `dirs::*`, but I haven't installed an actual session there yet.
- The codex/claude/opencode adapters still rely on agent installs
  putting their state under `~/.codex/`, `~/.claude/`, and
  `~/.local/share/opencode/` respectively. If your distro/agent uses a
  different XDG layout, set `CODEX_PETS_DIR` or open an adapter PR.

#### Windows (untested)

```powershell
# Install prerequisites once:
#   - Visual Studio Build Tools (Desktop development with C++)
#   - WebView2 Evergreen runtime (most Win10/11 installs already have it)
#   - Rust (rustup-init.exe)
#   - Node 20+
git clone <repo>
cd codex-pet-desktop
npm install
npm run tauri:build                     # → src-tauri\target\release\bundle\{msi,nsis}\
```

Likely working but not verified by the maintainer:
- The macOS-specific window code (panel-style, Stage Manager bypass)
  is `#[cfg(target_os = "macos")]`-gated; on Windows you get a normal
  always-on-top transparent layered window.
- `find_process_by_name` uses `sysinfo`'s NtQuerySystemInformation
  backend on Windows. Should work, but agent process names may differ
  (`codex.exe` vs `codex`, etc.).
- Pet sprites under `%USERPROFILE%\.codex\pets\` and the cache under
  `%LOCALAPPDATA%\codex-pet-desktop\` are derived from `dirs::*`; the
  Codex.app asar discovery is gated to macOS only.

If you build on either platform and something works (or doesn't),
please [open an issue or PR](#contributing) so we can mark it verified
or fix the rough edges.

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
