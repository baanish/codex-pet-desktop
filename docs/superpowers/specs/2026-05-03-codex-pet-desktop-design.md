# Codex Pet Desktop — Design Doc

## Overview

A lightweight, cross-platform Electron desktop app that displays an animated codex pet as a transparent overlay. The pet reacts to active coding sessions across OpenCode, Claude Code, and Codex, changing its animation based on thread state and surfacing the latest thread title.

## Goals

- Display any codex pet from `~/.codex/pets/<pet-id>/` as a transparent always-on-top overlay
- Detect active threads in OpenCode, Claude Code, and Codex in real-time
- Map thread states to pet animations (idle, running, waiting, failed, waving)
- Show the latest thread title for each active tool via a floating label
- Draggable pet with persistent position across launches
- Minimal footprint — vanilla TS + Canvas, no frontend framework

## Non-Goals

- Pet creation/hatching (use the hatch-pet skill for that)
- Notification system or click-to-focus tool integration
- Multi-pet display (one pet at a time)
- Windows/Linux system tray integration (future enhancement)

## Architecture

### Two-Process Model

```
┌─────────────────────────────────────────────┐
│ Main Process (Node.js)                      │
│  ├─ PetLoader                               │
│  ├─ ThreadMonitor                           │
│  │   └─ adapters/                           │
│  │       ├─ adapter.ts        (interface)   │
│  │       ├─ opencode.ts                    │
│  │       ├─ claude-code.ts                 │
│  │       └─ codex.ts                       │
│  ├─ ConfigStore                             │
│  └─ WindowManager                           │
└──────────────────┬──────────────────────────┘
                   IPC
┌──────────────────▼──────────────────────────┐
│ Renderer Process (Chromium)                 │
│  ├─ SpriteEngine (Canvas 2D)                │
│  ├─ ThreadLabel (DOM overlay)               │
│  └─ DragHandler (pointer events)            │
└─────────────────────────────────────────────┘
```

### Pluggable Adapter System

Every tool detector implements the same interface. Adding a new tool means adding one file.

```ts
interface ThreadAdapter {
  readonly id: string
  readonly displayName: string
  isInstalled(): boolean
  poll(): Promise<ActiveThread[]>
  destroy(): void
}

interface ActiveThread {
  tool: string
  status: ThreadStatus
  title: string | null
}

type ThreadStatus =
  | 'busy'
  | 'idle'
  | 'waiting'
  | 'error'
  | 'stale'
```

### Poll Behavior

- **Default interval**: 30 seconds (configurable via settings)
- **On-interaction polling**: When the user drags or clicks the pet, `ThreadMonitor.triggerPoll()` fires an immediate poll
- **Poll interval stored in config**: `{ pollIntervalMs: 30000, ... }`

### Adapter Details

#### OpenCodeAdapter
- Check `~/.local/share/opencode/opencode.db-wal` size > 0
- Confirm process alive via `pgrep` or `ps aux`
- Query SQLite: `SELECT title FROM session WHERE time_archived IS NULL ORDER BY time_updated DESC LIMIT 1`

#### ClaudeCodeAdapter
- Read `~/.claude/sessions/*.json` files
- Primary: check `pid` field in JSON body is alive
- Fallback: if file modified within last 60s, treat as stale
- Extract `name` and `status` fields

#### CodexAdapter
- Check `~/.codex/tmp/arg0/codex-*/.lock` files
- Use `lsof` with fallback to `ps aux grep` for lock detection
- Query SQLite: `SELECT title FROM threads WHERE archived=0 ORDER BY updated_at DESC LIMIT 1`

### Main Process

#### PetLoader
- Scans `~/.codex/pets/` for directories containing `pet.json` + `spritesheet.webp`
- Reads `pet.json` to get id, displayName, description, spritesheetPath

#### ConfigStore
- Location: `app.getPath('userData')` → `~/.config/codex-pet-desktop/config.json`
- Stores: `{ selectedPetId, position, alwaysOnTop, pollIntervalMs }`
- Debounced writes (500ms)

#### WindowManager
- `BrowserWindow` with: `transparent: true`, `frame: false`, `alwaysOnTop: true`, `hasShadow: false`, `resizable: false`, `skipTaskbar: true`
- Window size = sprite cell size (192×208) + label height (~40px)

#### Graceful Shutdown
```ts
app.on('before-quit', () => {
  monitor.destroy()
  config.flush()
})
```

### Renderer Process

#### SpriteEngine
- Canvas 2D `drawImage()` with source clipping rect per frame
- `requestAnimationFrame` loop
- Atlas: 8×9 grid, 192×208 cells

#### Animation State Mapping
```
No active threads           → idle (loop)
Any thread status 'busy'    → running (loop)
Any thread status 'waiting' → waiting (loop)
Any thread status 'error'   → failed (loop)
Any thread status 'stale'   → idle (loop, greyed label)
Thread completed (brief)    → waving (once, then idle)
Priority: error > busy > waiting > stale > idle
```

#### ThreadLabel
- DOM element above pet canvas
- Shows `[tool name] — thread title` per active tool
- Stale entries shown with reduced opacity (0.5)
- Hidden when no active threads

#### DragHandler
- `pointerdown` → capture offset, trigger immediate poll
- `pointermove` → update position, clamp to screen
- `pointerup` → save to config

## File Structure

```
codex-pet-desktop/
├── package.json
├── tsconfig.json
├── electron-builder.yml
├── src/
│   ├── main/
│   │   ├── index.ts
│   │   ├── window.ts
│   │   ├── pet-loader.ts
│   │   ├── config.ts
│   │   └── threads/
│   │       ├── monitor.ts
│   │       ├── adapter.ts
│   │       ├── opencode.ts
│   │       ├── claude-code.ts
│   │       └── codex.ts
│   └── renderer/
│       ├── index.html
│       ├── main.ts
│       ├── sprite-engine.ts
│       ├── thread-label.ts
│       ├── drag-handler.ts
│       ├── atlas.ts
│       └── types.ts
└── .gitignore
```

## Dependencies

```json
{
  "dependencies": {
    "better-sqlite3": "^11.x"
  },
  "devDependencies": {
    "electron": "^33.x",
    "electron-builder": "^25.x",
    "typescript": "^5.6.x"
  }
}
```

## Risks & Mitigations

| Risk                                                 | Mitigation                                                         |
| ---------------------------------------------------- | ------------------------------------------------------------------ |
| SQLite DB locked by active process                   | Open with `readonly: true` + WAL mode; wrap in try/catch             |
| `lsof` unavailable on some platforms                   | `isLockHeld()` falls back to `ps aux grep`                           |
| Claude Code format changes                           | Read `pid` from JSON body, mtime fallback                            |
| Thread title extraction fails                        | Return `null` for title, still detect active state                   |
| Stale state persists                                 | `stale` status with visual indicator; auto-clears on next clean poll |
| App killed without before-quit                       | Config debounced-written; SQLite handles are non-blocking           |
