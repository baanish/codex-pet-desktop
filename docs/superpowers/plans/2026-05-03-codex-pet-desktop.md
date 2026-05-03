# Codex Pet Desktop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a cross-platform Electron desktop app that displays an animated codex pet as a transparent overlay, reacting to active threads in OpenCode, Claude Code, and Codex.

**Architecture:** Two-process Electron app — main process handles thread monitoring and window management, renderer process handles Canvas-based sprite animation and thread label UI. Pluggable adapter system for tool detection.

**Tech Stack:** Electron 33, TypeScript 5.6, better-sqlite3, Canvas 2D, electron-builder

---

## File Structure

```
codex-pet-desktop/
├── package.json
├── tsconfig.json
├── tsconfig.node.json
├── electron-builder.yml
├── .gitignore
├── src/
│   ├── shared/
│   │   └── types.ts
│   ├── main/
│   │   ├── index.ts
│   │   ├── window.ts
│   │   ├── pet-loader.ts
│   │   ├── config.ts
│   │   └── threads/
│   │       ├── adapter.ts
│   │       ├── monitor.ts
│   │       ├── opencode.ts
│   │       ├── claude-code.ts
│   │       └── codex.ts
│   └── renderer/
│       ├── index.html
│       ├── main.ts
│       ├── sprite-engine.ts
│       ├── thread-label.ts
│       ├── drag-handler.ts
│       └── atlas.ts
```

---

### Task 1: Project Scaffolding

**Files:**
- Create: `package.json`
- Create: `tsconfig.json`
- Create: `tsconfig.node.json`
- Create: `electron-builder.yml`
- Create: `.gitignore`

- [ ] **Step 1: Create package.json**

```json
{
  "name": "codex-pet-desktop",
  "version": "1.0.0",
  "description": "Animated codex pet desktop overlay",
  "main": "dist/main/index.js",
  "scripts": {
    "dev": "tsc -p tsconfig.node.json && electron .",
    "build": "tsc -p tsconfig.node.json && tsc -p tsconfig.json",
    "dist": "npm run build && electron-builder",
    "typecheck": "tsc --noEmit -p tsconfig.node.json && tsc --noEmit -p tsconfig.json"
  },
  "dependencies": {
    "better-sqlite3": "^11.0.0"
  },
  "devDependencies": {
    "@types/better-sqlite3": "^7.6.0",
    "electron": "^33.0.0",
    "electron-builder": "^25.0.0",
    "typescript": "^5.6.0"
  }
}
```

- [ ] **Step 2: Create tsconfig.json (renderer)**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ES2022",
    "moduleResolution": "bundler",
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "outDir": "dist/renderer",
    "rootDir": "src/renderer",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "declaration": false,
    "sourceMap": true
  },
  "include": ["src/renderer/**/*.ts", "src/shared/**/*.ts"]
}
```

- [ ] **Step 3: Create tsconfig.node.json (main process)**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "commonjs",
    "moduleResolution": "node",
    "lib": ["ES2022"],
    "outDir": "dist/main",
    "rootDir": "src/main",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "declaration": false,
    "sourceMap": true
  },
  "include": ["src/main/**/*.ts", "src/shared/**/*.ts"]
}
```

- [ ] **Step 4: Create electron-builder.yml**

```yaml
appId: com.codex-pet.desktop
productName: Codex Pet Desktop
directories:
  output: release
files:
  - dist/**/*
  - package.json
mac:
  target: [dmg, zip]
  category: public.app-category.developer-tools
win:
  target: [nsis]
linux:
  target: [AppImage, deb]
```

- [ ] **Step 5: Create .gitignore**

```
node_modules/
dist/
release/
*.js.map
.DS_Store
```

- [ ] **Step 6: Install dependencies**

Run: `npm install`

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "chore: scaffold project with electron, typescript, better-sqlite3"
```

---

### Task 2: Shared Types

**Files:**
- Create: `src/shared/types.ts`

- [ ] **Step 1: Write shared types**

```ts
export interface PetJson {
  id: string
  displayName: string
  description: string
  spritesheetPath: string
}

export interface PetInfo extends PetJson {
  directory: string
  spritesheetAbsPath: string
}

export interface SpriteAnimation {
  row: number
  frames: number
  frameDurations: number[]
}

export interface PetSpriteAtlas {
  columns: number
  rows: number
  cellWidth: number
  cellHeight: number
  animations: Record<string, SpriteAnimation>
}

export type ThreadStatus = 'busy' | 'idle' | 'waiting' | 'error' | 'stale'

export interface ActiveThread {
  tool: string
  status: ThreadStatus
  title: string | null
}

export interface AppState {
  activeThreads: ActiveThread[]
}

export interface AppConfig {
  selectedPetId: string | null
  position: { x: number; y: number }
  alwaysOnTop: boolean
  pollIntervalMs: number
}

export const DEFAULT_CONFIG: AppConfig = {
  selectedPetId: null,
  position: { x: 0, y: 0 },
  alwaysOnTop: true,
  pollIntervalMs: 30000
}
```

- [ ] **Step 2: Verify types compile**

Run: `npx tsc --noEmit -p tsconfig.node.json && npx tsc --noEmit -p tsconfig.json`

- [ ] **Step 3: Commit**

```bash
git add src/shared/types.ts
git commit -m "feat: add shared types for pet, thread, and config"
```

---

### Task 3: Adapter Interface and Helpers

**Files:**
- Create: `src/main/threads/adapter.ts`

- [ ] **Step 1: Write adapter interface and helpers**

```ts
import { execSync } from 'child_process'
import { ActiveThread } from '../../shared/types'

export interface ThreadAdapter {
  readonly id: string
  readonly displayName: string
  isInstalled(): boolean
  poll(): Promise<ActiveThread[]>
  destroy(): void
}

export function isPidAlive(pid: number): boolean {
  try {
    process.kill(pid, 0)
    return true
  } catch {
    return false
  }
}

export function findProcessByName(pattern: string): number | null {
  try {
    const result = execSync(`pgrep -f "${pattern}"`, {
      encoding: 'utf-8',
      timeout: 5000,
      stdio: ['pipe', 'pipe', 'pipe']
    })
    const pids = result.trim().split('\n').filter(Boolean)
    if (pids.length > 0) {
      return parseInt(pids[0], 10)
    }
  } catch {
    // pgrep returns exit 1 if no match
  }
  return null
}

export function isLockHeldByProcess(lockPath: string): boolean {
  try {
    const result = execSync(`lsof "${lockPath}"`, {
      encoding: 'utf-8',
      timeout: 5000,
      stdio: ['pipe', 'pipe', 'pipe']
    })
    return result.trim().length > 0
  } catch {
    return false
  }
}
```

- [ ] **Step 2: Verify types compile**

Run: `npx tsc --noEmit -p tsconfig.node.json`

- [ ] **Step 3: Commit**

```bash
git add src/main/threads/adapter.ts
git commit -m "feat: add thread adapter interface and process detection helpers"
```

---

### Task 4: OpenCode Adapter

**Files:**
- Create: `src/main/threads/opencode.ts`

- [ ] **Step 1: Write OpenCode adapter**

```ts
import { existsSync, statSync } from 'fs'
import { join } from 'path'
import { homedir } from 'os'
import { ThreadAdapter, findProcessByName } from './adapter'
import { ActiveThread } from '../../shared/types'

export class OpenCodeAdapter implements ThreadAdapter {
  readonly id = 'opencode'
  readonly displayName = 'OpenCode'

  private dbPath: string
  private walPath: string

  constructor() {
    const dataDir = join(homedir(), '.local/share/opencode')
    this.dbPath = join(dataDir, 'opencode.db')
    this.walPath = join(dataDir, 'opencode.db-wal')
  }

  isInstalled(): boolean {
    return existsSync(this.dbPath)
  }

  async poll(): Promise<ActiveThread[]> {
    try {
      // Check WAL file — if missing or empty, tool is not running
      if (!existsSync(this.walPath)) return []
      const walStat = statSync(this.walPath)
      if (walStat.size === 0) return []

      // Confirm process is alive
      const pid = findProcessByName('opencode')
      if (!pid) {
        return [{ tool: this.id, status: 'stale', title: null }]
      }

      // Query SQLite read-only
      // eslint-disable-next-line @typescript-eslint/no-var-requires
      const Database = require('better-sqlite3')
      const db = new Database(this.dbPath, {
        readonly: true,
        fileMustExist: true
      })

      try {
        const row = db.prepare(
          'SELECT title FROM session WHERE time_archived IS NULL ORDER BY time_updated DESC LIMIT 1'
        ).get() as { title: string | null } | undefined

        return [{
          tool: this.id,
          status: 'busy',
          title: row?.title ?? null
        }]
      } finally {
        db.close()
      }
    } catch {
      return []
    }
  }

  destroy(): void {}
}
```

- [ ] **Step 2: Verify types compile**

Run: `npx tsc --noEmit -p tsconfig.node.json`

- [ ] **Step 3: Commit**

```bash
git add src/main/threads/opencode.ts
git commit -m "feat: add OpenCode thread adapter"
```

---

### Task 5: Claude Code Adapter

**Files:**
- Create: `src/main/threads/claude-code.ts`

- [ ] **Step 1: Write Claude Code adapter**

```ts
import { existsSync, readdirSync, readFileSync, statSync } from 'fs'
import { join } from 'path'
import { homedir } from 'os'
import { ThreadAdapter, isPidAlive } from './adapter'
import { ActiveThread } from '../../shared/types'

interface ClaudeSessionFile {
  pid?: number
  name?: string
  status?: string
  updatedAt?: number
}

export class ClaudeCodeAdapter implements ThreadAdapter {
  readonly id = 'claude-code'
  readonly displayName = 'Claude Code'

  private sessionsDir: string

  constructor() {
    this.sessionsDir = join(homedir(), '.claude/sessions')
  }

  isInstalled(): boolean {
    return existsSync(join(homedir(), '.claude'))
  }

  async poll(): Promise<ActiveThread[]> {
    if (!existsSync(this.sessionsDir)) return []

    try {
      const files = readdirSync(this.sessionsDir).filter(f => f.endsWith('.json'))
      const active: ActiveThread[] = []

      for (const file of files) {
        const filePath = join(this.sessionsDir, file)
        try {
          const data: ClaudeSessionFile = JSON.parse(readFileSync(filePath, 'utf-8'))

          // Primary: check if pid field in JSON is alive
          if (data.pid && isPidAlive(data.pid)) {
            active.push({
              tool: this.id,
              status: data.status === 'busy' ? 'busy' : 'idle',
              title: data.name ?? null
            })
            continue
          }

          // Fallback: if file was modified recently (< 60s), treat as stale
          const mtime = statSync(filePath).mtimeMs
          if (Date.now() - mtime < 60_000) {
            active.push({
              tool: this.id,
              status: 'stale',
              title: data.name ?? null
            })
          }
        } catch {
          // Skip malformed files
        }
      }

      return active
    } catch {
      return []
    }
  }

  destroy(): void {}
}
```

- [ ] **Step 2: Verify types compile**

Run: `npx tsc --noEmit -p tsconfig.node.json`

- [ ] **Step 3: Commit**

```bash
git add src/main/threads/claude-code.ts
git commit -m "feat: add Claude Code thread adapter with PID + mtime fallback"
```

---

### Task 6: Codex Adapter

**Files:**
- Create: `src/main/threads/codex.ts`

- [ ] **Step 1: Write Codex adapter**

```ts
import { existsSync, readdirSync, statSync } from 'fs'
import { join } from 'path'
import { homedir } from 'os'
import { ThreadAdapter, isLockHeldByProcess, findProcessByName } from './adapter'
import { ActiveThread } from '../../shared/types'

export class CodexAdapter implements ThreadAdapter {
  readonly id = 'codex'
  readonly displayName = 'Codex'

  private codexDir: string
  private dbPath: string

  constructor() {
    this.codexDir = join(homedir(), '.codex')
    this.dbPath = join(this.codexDir, 'state_5.sqlite')
  }

  isInstalled(): boolean {
    return existsSync(this.codexDir)
  }

  async poll(): Promise<ActiveThread[]> {
    try {
      // Check for held lock files
      const locksDir = join(this.codexDir, 'tmp/arg0')
      if (!existsSync(locksDir)) return []

      const lockDirs = readdirSync(locksDir).filter(d => d.startsWith('codex-'))
      let hasLiveLock = false

      for (const dir of lockDirs) {
        const lockFile = join(locksDir, dir, '.lock')
        if (existsSync(lockFile)) {
          if (isLockHeldByProcess(lockFile)) {
            hasLiveLock = true
            break
          }
        }
      }

      // Fallback: check for codex process
      if (!hasLiveLock) {
        const pid = findProcessByName('codex')
        if (!pid) return []
        hasLiveLock = true
      }

      // Query SQLite for latest thread
      if (!existsSync(this.dbPath)) {
        return [{ tool: this.id, status: 'busy', title: null }]
      }

      // eslint-disable-next-line @typescript-eslint/no-var-requires
      const Database = require('better-sqlite3')
      const db = new Database(this.dbPath, {
        readonly: true,
        fileMustExist: true
      })

      try {
        const row = db.prepare(
          'SELECT title FROM threads WHERE archived=0 ORDER BY updated_at DESC LIMIT 1'
        ).get() as { title: string | null } | undefined

        return [{
          tool: this.id,
          status: 'busy',
          title: row?.title ?? null
        }]
      } finally {
        db.close()
      }
    } catch {
      return []
    }
  }

  destroy(): void {}
}
```

- [ ] **Step 2: Verify types compile**

Run: `npx tsc --noEmit -p tsconfig.node.json`

- [ ] **Step 3: Commit**

```bash
git add src/main/threads/codex.ts
git commit -m "feat: add Codex thread adapter with lsof + ps fallback"
```

---

### Task 7: Thread Monitor

**Files:**
- Create: `src/main/threads/monitor.ts`

- [ ] **Step 1: Write ThreadMonitor**

```ts
import { ThreadAdapter, ActiveThread } from '../../shared/types'

export class ThreadMonitor {
  private adapters: ThreadAdapter[]
  private interval: ReturnType<typeof setInterval> | null = null
  private onState: (threads: ActiveThread[]) => void

  constructor(adapters: ThreadAdapter[], onState: (threads: ActiveThread[]) => void) {
    this.adapters = adapters.filter(a => a.isInstalled())
    this.onState = onState
  }

  start(intervalMs: number) {
    const poll = async () => {
      try {
        const results = await Promise.allSettled(
          this.adapters.map(a => a.poll())
        )
        const threads = results
          .filter((r): r is PromiseFulfilledResult<ActiveThread[]> => r.status === 'fulfilled')
          .flatMap(r => r.value)
        this.onState(threads)
      } catch {
        // Don't crash on poll errors
      }
    }

    this.interval = setInterval(poll, intervalMs)
    poll() // initial poll
  }

  triggerPoll() {
    // Immediate poll on interaction
    const poll = async () => {
      try {
        const results = await Promise.allSettled(
          this.adapters.map(a => a.poll())
        )
        const threads = results
          .filter((r): r is PromiseFulfilledResult<ActiveThread[]> => r.status === 'fulfilled')
          .flatMap(r => r.value)
        this.onState(threads)
      } catch {
        // Don't crash on poll errors
      }
    }
    poll()
  }

  destroy() {
    if (this.interval) {
      clearInterval(this.interval)
      this.interval = null
    }
    this.adapters.forEach(a => a.destroy())
  }
}
```

- [ ] **Step 2: Verify types compile**

Run: `npx tsc --noEmit -p tsconfig.node.json`

- [ ] **Step 3: Commit**

```bash
git add src/main/threads/monitor.ts
git commit -m "feat: add ThreadMonitor with pluggable adapters and poll control"
```

---

### Task 8: Config Store

**Files:**
- Create: `src/main/config.ts`

- [ ] **Step 1: Write ConfigStore**

```ts
import { existsSync, readFileSync, writeFileSync, mkdirSync } from 'fs'
import { dirname, join } from 'path'
import { AppConfig, DEFAULT_CONFIG } from '../shared/types'

export class ConfigStore {
  private path: string
  private config: AppConfig
  private writeTimeout: ReturnType<typeof setTimeout> | null = null

  constructor(userDataPath: string) {
    this.path = join(userDataPath, 'config.json')
    this.config = this.load()
  }

  private load(): AppConfig {
    try {
      if (existsSync(this.path)) {
        const raw = readFileSync(this.path, 'utf-8')
        const parsed = JSON.parse(raw)
        return { ...DEFAULT_CONFIG, ...parsed }
      }
    } catch {
      // Corrupted config, use defaults
    }
    return { ...DEFAULT_CONFIG }
  }

  get(): AppConfig {
    return { ...this.config }
  }

  update(partial: Partial<AppConfig>) {
    this.config = { ...this.config, ...partial }
    this.scheduleWrite()
  }

  private scheduleWrite() {
    if (this.writeTimeout) {
      clearTimeout(this.writeTimeout)
    }
    this.writeTimeout = setTimeout(() => {
      this.flush()
    }, 500)
  }

  flush() {
    if (this.writeTimeout) {
      clearTimeout(this.writeTimeout)
      this.writeTimeout = null
    }
    try {
      const dir = dirname(this.path)
      if (!existsSync(dir)) {
        mkdirSync(dir, { recursive: true })
      }
      writeFileSync(this.path, JSON.stringify(this.config, null, 2), 'utf-8')
    } catch {
      // Don't crash on write errors
    }
  }
}
```

- [ ] **Step 2: Verify types compile**

Run: `npx tsc --noEmit -p tsconfig.node.json`

- [ ] **Step 3: Commit**

```bash
git add src/main/config.ts
git commit -m "feat: add ConfigStore with debounced writes"
```

---

### Task 9: Pet Loader

**Files:**
- Create: `src/main/pet-loader.ts`

- [ ] **Step 1: Write PetLoader**

```ts
import { existsSync, readdirSync, readFileSync } from 'fs'
import { join } from 'path'
import { homedir } from 'os'
import { PetInfo, PetJson } from '../shared/types'

export function loadPets(): PetInfo[] {
  const petsDir = join(homedir(), '.codex/pets')
  if (!existsSync(petsDir)) return []

  const entries = readdirSync(petsDir, { withFileTypes: true })
  const pets: PetInfo[] = []

  for (const entry of entries) {
    if (!entry.isDirectory()) continue

    const petDir = join(petsDir, entry.name)
    const jsonPath = join(petDir, 'pet.json')
    const spritesheetPath = join(petDir, 'spritesheet.webp')

    if (!existsSync(jsonPath) || !existsSync(spritesheetPath)) continue

    try {
      const raw = readFileSync(jsonPath, 'utf-8')
      const json: PetJson = JSON.parse(raw)
      pets.push({
        ...json,
        directory: petDir,
        spritesheetAbsPath: spritesheetPath
      })
    } catch {
      // Skip malformed pet.json
    }
  }

  return pets
}
```

- [ ] **Step 2: Verify types compile**

Run: `npx tsc --noEmit -p tsconfig.node.json`

- [ ] **Step 3: Commit**

```bash
git add src/main/pet-loader.ts
git commit -m "feat: add PetLoader to scan ~/.codex/pets/"
```

---

### Task 10: Window Manager

**Files:**
- Create: `src/main/window.ts`

- [ ] **Step 1: Write WindowManager**

```ts
import { BrowserWindow, screen } from 'electron'
import { join } from 'path'
import { AppConfig } from '../shared/types'

const CELL_WIDTH = 192
const CELL_HEIGHT = 208
const LABEL_HEIGHT = 50
const WINDOW_WIDTH = CELL_WIDTH
const WINDOW_HEIGHT = CELL_HEIGHT + LABEL_HEIGHT

export function createPetWindow(config: AppConfig): BrowserWindow {
  const { position, alwaysOnTop } = config

  // Clamp position to screen bounds
  const display = screen.getPrimaryDisplay()
  const { width: screenW, height: screenH } = display.workAreaSize
  const x = Math.max(0, Math.min(position.x, screenW - WINDOW_WIDTH))
  const y = Math.max(0, Math.min(position.y, screenH - WINDOW_HEIGHT))

  const win = new BrowserWindow({
    width: WINDOW_WIDTH,
    height: WINDOW_HEIGHT,
    x,
    y,
    transparent: true,
    frame: false,
    alwaysOnTop,
    hasShadow: false,
    resizable: false,
    skipTaskbar: true,
    backgroundColor: '#00000000',
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      preload: join(__dirname, 'preload.js')
    }
  })

  win.setVisibleOnAllWorkspaces(true, { visibleOnFullScreen: true })

  // Prevent window from being focused
  win.on('focus', () => {
    // Allow focus for drag interactions
  })

  return win
}
```

- [ ] **Step 2: Create preload script**

Create: `src/main/preload.ts`

```ts
import { contextBridge, ipcRenderer } from 'electron'

contextBridge.exposeInMainWorld('petBridge', {
  onThreadState: (callback: (data: any) => void) => {
    ipcRenderer.on('thread-state', (_event, data) => callback(data))
  },
  onPetData: (callback: (data: any) => void) => {
    ipcRenderer.on('pet-data', (_event, data) => callback(data))
  },
  onConfig: (callback: (data: any) => void) => {
    ipcRenderer.on('config', (_event, data) => callback(data))
  },
  savePosition: (position: { x: number; y: number }) => {
    ipcRenderer.send('save-position', position)
  },
  triggerPoll: () => {
    ipcRenderer.send('trigger-poll')
  }
})
```

- [ ] **Step 3: Update tsconfig.node.json to include preload**

Update `tsconfig.node.json` include to also cover `src/main/preload.ts`.

- [ ] **Step 4: Verify types compile**

Run: `npx tsc --noEmit -p tsconfig.node.json`

- [ ] **Step 5: Commit**

```bash
git add src/main/window.ts src/main/preload.ts
git commit -m "feat: add WindowManager and preload script for IPC bridge"
```

---

### Task 11: Atlas Definition

**Files:**
- Create: `src/renderer/atlas.ts`

- [ ] **Step 1: Write atlas definition**

```ts
import { PetSpriteAtlas } from '../shared/types'

export type CodexPetAnimationName =
  | 'idle'
  | 'running-right'
  | 'running-left'
  | 'waving'
  | 'jumping'
  | 'failed'
  | 'waiting'
  | 'running'
  | 'review'

export const codexPetAtlas: PetSpriteAtlas = {
  columns: 8,
  rows: 9,
  cellWidth: 192,
  cellHeight: 208,
  animations: {
    idle: {
      row: 0,
      frames: 6,
      frameDurations: [280, 110, 110, 140, 140, 320]
    },
    'running-right': {
      row: 1,
      frames: 8,
      frameDurations: [120, 120, 120, 120, 120, 120, 120, 220]
    },
    'running-left': {
      row: 2,
      frames: 8,
      frameDurations: [120, 120, 120, 120, 120, 120, 120, 220]
    },
    waving: {
      row: 3,
      frames: 4,
      frameDurations: [140, 140, 140, 280]
    },
    jumping: {
      row: 4,
      frames: 5,
      frameDurations: [140, 140, 140, 140, 280]
    },
    failed: {
      row: 5,
      frames: 8,
      frameDurations: [140, 140, 140, 140, 140, 140, 140, 240]
    },
    waiting: {
      row: 6,
      frames: 6,
      frameDurations: [150, 150, 150, 150, 150, 260]
    },
    running: {
      row: 7,
      frames: 6,
      frameDurations: [120, 120, 120, 120, 120, 220]
    },
    review: {
      row: 8,
      frames: 6,
      frameDurations: [150, 150, 150, 150, 150, 280]
    }
  }
}
```

- [ ] **Step 2: Verify types compile**

Run: `npx tsc --noEmit -p tsconfig.json`

- [ ] **Step 3: Commit**

```bash
git add src/renderer/atlas.ts
git commit -m "feat: add codex pet atlas definition matching spritesheet layout"
```

---

### Task 12: Sprite Engine

**Files:**
- Create: `src/renderer/sprite-engine.ts`

- [ ] **Step 1: Write SpriteEngine**

```ts
import { PetSpriteAtlas, SpriteAnimation } from '../shared/types'

export class SpriteEngine {
  private canvas: HTMLCanvasElement
  private ctx: CanvasRenderingContext2D
  private image: HTMLImageElement
  private atlas: PetSpriteAtlas
  private currentAnimation: SpriteAnimation | null = null
  private currentFrame = 0
  private elapsedMs = 0
  private lastTimestamp = 0
  private running = false

  constructor(canvas: HTMLCanvasElement, atlas: PetSpriteAtlas) {
    this.canvas = canvas
    this.ctx = canvas.getContext('2d')!
    this.atlas = atlas
    this.image = new Image()

    canvas.width = atlas.cellWidth
    canvas.height = atlas.cellHeight
  }

  loadSpritesheet(src: string): Promise<void> {
    return new Promise((resolve, reject) => {
      this.image.onload = () => resolve()
      this.image.onerror = reject
      this.image.src = src
    })
  }

  setAnimation(name: string) {
    const anim = this.atlas.animations[name]
    if (!anim) return
    if (this.currentAnimation === anim) return

    this.currentAnimation = anim
    this.currentFrame = 0
    this.elapsedMs = 0
  }

  start() {
    if (this.running) return
    this.running = true
    this.lastTimestamp = performance.now()
    this.tick()
  }

  stop() {
    this.running = false
  }

  private tick = () => {
    if (!this.running) return

    const now = performance.now()
    const delta = now - this.lastTimestamp
    this.lastTimestamp = now

    this.elapsedMs += delta

    if (this.currentAnimation) {
      const frameDuration = this.currentAnimation.frameDurations[this.currentFrame]
      if (this.elapsedMs >= frameDuration) {
        this.elapsedMs -= frameDuration
        this.currentFrame = (this.currentFrame + 1) % this.currentAnimation.frames
      }

      this.draw()
    }

    requestAnimationFrame(this.tick)
  }

  private draw() {
    if (!this.currentAnimation) return

    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height)

    const { cellWidth, cellHeight } = this.atlas
    const col = this.currentFrame
    const row = this.currentAnimation.row

    this.ctx.drawImage(
      this.image,
      col * cellWidth,
      row * cellHeight,
      cellWidth,
      cellHeight,
      0,
      0,
      cellWidth,
      cellHeight
    )
  }
}
```

- [ ] **Step 2: Verify types compile**

Run: `npx tsc --noEmit -p tsconfig.json`

- [ ] **Step 3: Commit**

```bash
git add src/renderer/sprite-engine.ts
git commit -m "feat: add SpriteEngine with Canvas 2D frame animation"
```

---

### Task 13: Thread Label

**Files:**
- Create: `src/renderer/thread-label.ts`

- [ ] **Step 1: Write ThreadLabel**

```ts
import { ActiveThread, ThreadStatus } from '../shared/types'

const STATUS_PRIORITY: Record<ThreadStatus, number> = {
  error: 4,
  busy: 3,
  waiting: 2,
  stale: 1,
  idle: 0
}

export class ThreadLabel {
  private container: HTMLElement
  private labelEl: HTMLElement

  constructor(container: HTMLElement) {
    this.container = container
    this.labelEl = document.createElement('div')
    this.labelEl.style.cssText = `
      position: absolute;
      bottom: 100%;
      left: 50%;
      transform: translateX(-50%);
      background: rgba(0, 0, 0, 0.8);
      color: #fff;
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
      font-size: 11px;
      padding: 4px 8px;
      border-radius: 6px;
      white-space: nowrap;
      max-width: 250px;
      overflow: hidden;
      text-overflow: ellipsis;
      pointer-events: none;
      margin-bottom: 4px;
      display: none;
      line-height: 1.4;
    `
    container.appendChild(this.labelEl)
  }

  update(threads: ActiveThread[]) {
    if (threads.length === 0) {
      this.labelEl.style.display = 'none'
      return
    }

    // Sort by priority, take highest
    const sorted = [...threads].sort(
      (a, b) => STATUS_PRIORITY[b.status] - STATUS_PRIORITY[a.status]
    )

    const lines = sorted.map(t => {
      const title = t.title ?? 'untitled'
      const opacity = t.status === 'stale' ? '0.5' : '1'
      return `<div style="opacity: ${opacity}">${this.escapeHtml(t.tool)} — ${this.escapeHtml(title)}</div>`
    })

    this.labelEl.innerHTML = lines.join('')
    this.labelEl.style.display = 'block'
  }

  private escapeHtml(text: string): string {
    const div = document.createElement('div')
    div.textContent = text
    return div.innerHTML
  }
}
```

- [ ] **Step 2: Verify types compile**

Run: `npx tsc --noEmit -p tsconfig.json`

- [ ] **Step 3: Commit**

```bash
git add src/renderer/thread-label.ts
git commit -m "feat: add ThreadLabel floating UI component"
```

---

### Task 14: Drag Handler

**Files:**
- Create: `src/renderer/drag-handler.ts`

- [ ] **Step 1: Write DragHandler**

```ts
export class DragHandler {
  private el: HTMLElement
  private isDragging = false
  private offsetX = 0
  private offsetY = 0
  private onDragEnd: (position: { x: number; y: number }) => void
  private onInteraction: () => void

  constructor(
    el: HTMLElement,
    onDragEnd: (position: { x: number; y: number }) => void,
    onInteraction: () => void
  ) {
    this.el = el
    this.onDragEnd = onDragEnd
    this.onInteraction = onInteraction
    this.el.style.cursor = 'grab'

    this.el.addEventListener('pointerdown', this.onPointerDown)
    window.addEventListener('pointermove', this.onPointerMove)
    window.addEventListener('pointerup', this.onPointerUp)
    window.addEventListener('pointercancel', this.onPointerUp)
  }

  private onPointerDown = (e: PointerEvent) => {
    if (e.button !== 0) return
    e.preventDefault()

    this.isDragging = true
    this.el.style.cursor = 'grabbing'

    const rect = this.el.getBoundingClientRect()
    this.offsetX = e.clientX - rect.left
    this.offsetY = e.clientY - rect.top

    this.onInteraction()
  }

  private onPointerMove = (e: PointerEvent) => {
    if (!this.isDragging) return

    const x = e.clientX - this.offsetX
    const y = e.clientY - this.offsetY

    // Clamp to screen bounds
    const maxX = window.innerWidth - this.el.offsetWidth
    const maxY = window.innerHeight - this.el.offsetHeight
    const clampedX = Math.max(0, Math.min(x, maxX))
    const clampedY = Math.max(0, Math.min(y, maxY))

    this.el.style.transform = `translate(${clampedX}px, ${clampedY}px)`
  }

  private onPointerUp = () => {
    if (!this.isDragging) return

    this.isDragging = false
    this.el.style.cursor = 'grab'

    const rect = this.el.getBoundingClientRect()
    this.onDragEnd({ x: rect.left, y: rect.top })
  }

  destroy() {
    this.el.removeEventListener('pointerdown', this.onPointerDown)
    window.removeEventListener('pointermove', this.onPointerMove)
    window.removeEventListener('pointerup', this.onPointerUp)
    window.removeEventListener('pointercancel', this.onPointerUp)
  }
}
```

- [ ] **Step 2: Verify types compile**

Run: `npx tsc --noEmit -p tsconfig.json`

- [ ] **Step 3: Commit**

```bash
git add src/renderer/drag-handler.ts
git commit -m "feat: add DragHandler with pointer events and screen clamping"
```

---

### Task 15: Renderer HTML

**Files:**
- Create: `src/renderer/index.html`

- [ ] **Step 1: Write HTML entry**

```html
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Codex Pet</title>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    html, body {
      width: 100%;
      height: 100%;
      background: transparent;
      overflow: hidden;
      -webkit-app-region: no-drag;
    }
    #pet-container {
      position: relative;
      width: 192px;
      height: 258px; /* 208 + 50 for label */
    }
    #pet-canvas {
      position: absolute;
      bottom: 0;
      left: 0;
      image-rendering: pixelated;
    }
  </style>
</head>
<body>
  <div id="pet-container">
    <canvas id="pet-canvas"></canvas>
  </div>
</body>
</html>
```

- [ ] **Step 2: Commit**

```bash
git add src/renderer/index.html
git commit -m "feat: add renderer HTML entry with transparent layout"
```

---

### Task 16: Renderer Entry

**Files:**
- Create: `src/renderer/main.ts`

- [ ] **Step 1: Write renderer entry**

```ts
import { SpriteEngine } from './sprite-engine'
import { ThreadLabel } from './thread-label'
import { DragHandler } from './drag-handler'
import { codexPetAtlas } from './atlas'
import { ActiveThread, PetInfo, AppConfig } from '../shared/types'

declare global {
  interface Window {
    petBridge: {
      onThreadState: (callback: (data: ActiveThread[]) => void) => void
      onPetData: (callback: (data: PetInfo) => void) => void
      onConfig: (callback: (data: AppConfig) => void) => void
      savePosition: (position: { x: number; y: number }) => void
      triggerPoll: () => void
    }
  }
}

let engine: SpriteEngine | null = null
let threadLabel: ThreadLabel | null = null
let dragHandler: DragHandler | null = null

const ANIMATION_MAP: Record<string, string> = {
  error: 'failed',
  busy: 'running',
  waiting: 'waiting',
  idle: 'idle',
  stale: 'idle'
}

function getHighestPriorityAnimation(threads: ActiveThread[]): string {
  if (threads.length === 0) return 'idle'

  const priority = { error: 4, busy: 3, waiting: 2, stale: 1, idle: 0 }
  const sorted = [...threads].sort((a, b) => priority[b.status] - priority[a.status])
  return ANIMATION_MAP[sorted[0].status] ?? 'idle'
}

async function init() {
  const container = document.getElementById('pet-container')!
  const canvas = document.getElementById('pet-canvas') as HTMLCanvasElement

  engine = new SpriteEngine(canvas, codexPetAtlas)
  threadLabel = new ThreadLabel(container)

  dragHandler = new DragHandler(
    container,
    (position) => {
      window.petBridge.savePosition(position)
    },
    () => {
      window.petBridge.triggerPoll()
    }
  )

  window.petBridge.onPetData(async (pet: PetInfo) => {
    if (engine) {
      await engine.loadSpritesheet(pet.spritesheetAbsPath)
      engine.setAnimation('idle')
      engine.start()
    }
  })

  window.petBridge.onThreadState((threads: ActiveThread[]) => {
    if (engine) {
      const anim = getHighestPriorityAnimation(threads)
      engine.setAnimation(anim)
    }
    if (threadLabel) {
      threadLabel.update(threads)
    }
  })

  window.petBridge.onConfig((config: AppConfig) => {
    // Position is set by main process via window bounds
    // No action needed here
  })
}

init().catch(console.error)
```

- [ ] **Step 2: Verify types compile**

Run: `npx tsc --noEmit -p tsconfig.json`

- [ ] **Step 3: Commit**

```bash
git add src/renderer/main.ts
git commit -m "feat: add renderer entry wiring sprite engine, label, and drag"
```

---

### Task 17: Main Process Entry

**Files:**
- Create: `src/main/index.ts`

- [ ] **Step 1: Write main process entry**

```ts
import { app, BrowserWindow, ipcMain } from 'electron'
import { join } from 'path'
import { createPetWindow } from './window'
import { loadPets } from './pet-loader'
import { ConfigStore } from './config'
import { ThreadMonitor } from './threads/monitor'
import { OpenCodeAdapter } from './threads/opencode'
import { ClaudeCodeAdapter } from './threads/claude-code'
import { CodexAdapter } from './threads/codex'
import { ActiveThread } from '../shared/types'

let mainWindow: BrowserWindow | null = null
let config: ConfigStore | null = null
let monitor: ThreadMonitor | null = null

function sendToRenderer(channel: string, data: unknown) {
  if (mainWindow && !mainWindow.isDestroyed()) {
    mainWindow.webContents.send(channel, data)
  }
}

app.whenReady().then(() => {
  config = new ConfigStore(app.getPath('userData'))

  const adapters = [
    new OpenCodeAdapter(),
    new ClaudeCodeAdapter(),
    new CodexAdapter()
  ]

  monitor = new ThreadMonitor(adapters, (threads: ActiveThread[]) => {
    sendToRenderer('thread-state', threads)
  })

  const appConfig = config.get()
  mainWindow = createPetWindow(appConfig)

  mainWindow.loadFile(join(__dirname, '../renderer/index.html'))

  mainWindow.webContents.on('did-finish-load', () => {
    // Send pet data
    const pets = loadPets()
    if (pets.length > 0) {
      const selected = pets.find(p => p.id === appConfig.selectedPetId) ?? pets[0]
      sendToRenderer('pet-data', selected)
    }

    // Send config
    sendToRenderer('config', appConfig)

    // Start monitoring
    monitor!.start(appConfig.pollIntervalMs)
  })

  // IPC handlers
  ipcMain.on('save-position', (_event, position: { x: number; y: number }) => {
    config!.update({ position })
  })

  ipcMain.on('trigger-poll', () => {
    monitor!.triggerPoll()
  })
})

app.on('before-quit', () => {
  if (monitor) {
    monitor.destroy()
    monitor = null
  }
  if (config) {
    config.flush()
    config = null
  }
})

app.on('window-all-closed', () => {
  app.quit()
})
```

- [ ] **Step 2: Verify types compile**

Run: `npx tsc --noEmit -p tsconfig.node.json`

- [ ] **Step 3: Commit**

```bash
git add src/main/index.ts
git commit -m "feat: add main process entry wiring all components"
```

---

### Task 18: Build Verification

- [ ] **Step 1: Full typecheck**

Run: `npm run typecheck`

Expected: No errors

- [ ] **Step 2: Full build**

Run: `npm run build`

Expected: Compiled files in `dist/main/` and `dist/renderer/`

- [ ] **Step 3: Verify dist structure**

Run: `ls -la dist/main/ && ls -la dist/renderer/`

Expected: `index.js`, `window.js`, `pet-loader.js`, `config.js`, `threads/` in main; `index.html`, `main.js`, `sprite-engine.js`, etc. in renderer

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "chore: verify full build compiles successfully"
```

---

### Task 19: Dev Run Test

- [ ] **Step 1: Run the app in dev mode**

Run: `npm run dev`

Expected: Electron window appears with a transparent overlay. If no pets exist in `~/.codex/pets/`, the window will be empty but should not crash.

- [ ] **Step 2: Verify no console errors**

Check the Electron DevTools console for errors.

- [ ] **Step 3: Commit any fixes if needed**

```bash
git add -A
git commit -m "fix: address any runtime issues found during dev test"
```

---

### Task 20: Preload Build Fix

The preload script needs to be compiled separately or bundled. Update the build process.

- [ ] **Step 1: Add preload to tsconfig.node.json**

Ensure `src/main/preload.ts` is in the include array.

- [ ] **Step 2: Update window.ts to use compiled preload path**

The preload path in `window.ts` should point to `join(__dirname, 'preload.js')` which will be in `dist/main/preload.js` after build.

- [ ] **Step 3: Rebuild and verify**

Run: `npm run build && npm run dev`

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "fix: ensure preload script compiles and loads correctly"
```
