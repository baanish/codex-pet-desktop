# Contributing

Thanks for taking a look. The repo is small enough to keep in your head — this
file is mostly a map.

## Setup

You'll need:

- **Rust** (stable, 1.77+) — install via [rustup.rs](https://rustup.rs/).
- **Node.js 18+** — for the renderer build.
- A C/C++ toolchain (the standard `xcode-select --install` on macOS,
  build-essential on Linux, MSVC on Windows).

```bash
git clone <repo>
cd codex-pet-desktop
npm install
```

## Layout

See the architecture section in [README.md](README.md). Two halves:

| Path        | Language    | Role                                               |
| ---         | ---         | ---                                                |
| `src/`      | TypeScript  | Frontend (DOM, canvas, drag, IPC bindings)         |
| `src-tauri/`| Rust        | Backend (window, menu, config, agent polling, IPC) |

The TypeScript half compiles via esbuild into `dist/renderer/bundle.js`. Tauri
bundles the contents of `dist/renderer/` into the Rust binary at
`cargo build` time.

## Running

```bash
npm run dev           # Tauri's dev runner (rebuilds Rust on Rust changes)
npm run build         # Just the renderer (esbuild + cachebust)
npm run tauri:build   # Full release build → .app + .dmg / .msi / .AppImage
```

When you change **only** the renderer, the dev binary won't pick it up until
you also rebuild Rust (Tauri bakes the frontend dist into the binary). Either
re-run `npm run dev`, or `touch src-tauri/src/lib.rs && cargo build` to force.

## Testing

There isn't a formal suite yet. The smoke test is:

1. Build (`npm run tauri:build`).
2. Launch the .app (or `cargo run`).
3. Verify the pet shows up and animates idle.
4. Start one of the agents (e.g. open a Claude Code session) and confirm
   the thread card appears with the correct title / status.
5. Drag the pet, expand the card, click through the menu options.

If you're working on the polling layer, the easiest way to verify each
adapter independently is the **Agents** submenu — disable two of three and
watch that one in isolation.

## Conventions

- **Rust**: standard `cargo fmt` defaults; clippy not yet wired up but PRs
  welcome to enforce it.
- **TypeScript**: no formatter is enforced; match the surrounding style. No
  state in modules without a clear reason — `main.ts` already has a couple of
  module-level lets we'd prefer to factor out.
- **Comments**: explain *why* on non-obvious tradeoffs (the Stage Manager
  bypass, the cursor polling, the bundled-frontend cache busting). Don't
  explain what the code does — names should carry that.
- **No magic JSON keys**: shared shapes live in
  [`src/shared/types.ts`](src/shared/types.ts) and
  [`src-tauri/src/types.rs`](src-tauri/src/types.rs). Keep them in sync (the
  Rust side uses `serde(rename_all = "camelCase")`).

## Adding a pet

Drop a directory under `~/.codex/pets/<id>/` with:

```
pet.json          # id, displayName, description, spritesheetPath
spritesheet.webp  # 8 columns × 9 rows × 192×208 cells
```

See [README.md](README.md#pets) for the row → animation mapping.

## Adding an agent adapter

1. Add a `<Name>Adapter` struct in `src-tauri/src/threads/` that implements
   the `ThreadAdapter` trait (look at the existing three for reference).
2. Register it in `lib.rs` next to the other adapters.
3. The agent will show up in the **Agents** submenu and `enabledAgents`
   config automatically.

The trait is small:

```rust
pub trait ThreadAdapter: Send + Sync {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn is_installed(&self) -> bool;
    fn poll(&self) -> Vec<ActiveThread>;
}
```

`is_installed` runs once at startup; adapters that return `false` are
skipped permanently. `poll` is called every `pollIntervalMs`; return one
`ActiveThread` per session/thread you can find.

## Branches

- `main` — current Tauri implementation. Default.
- `legacy-electron` — the original Electron build, archived for posterity.
  Bug fixes on the legacy branch are not expected.

## Debugging

Run with `PET_DEV_VISIBLE=1` to get a decorated window + devtools + on-screen
debug overlay. Useful for tracing IPC flow when things look invisible.

A diagnostic file `pet-diagnostic.json` is written at startup to the user
data directory with window bounds, activation policy, and Stage Manager
strip width — handy if the overlay isn't where you expect.

## License

By contributing you agree your contributions are licensed under the MIT
license (see [LICENSE](LICENSE)).
