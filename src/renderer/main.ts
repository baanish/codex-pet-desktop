import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { SpriteEngine } from './sprite-engine'
import { ThreadLabel } from './thread-label'
import { codexPetAtlas } from './atlas'
import { ActiveThread, PetInfo, AppConfig } from '../shared/types'

let engine: SpriteEngine | null = null
let threadLabel: ThreadLabel | null = null
let currentScale = 1
let debugAnimation: string | null = null
let lastThreadAnimation = 'idle'
let isDragging = false

const ANIMATION_MAP: Record<string, string> = {
  error: 'failed',
  busy: 'running',
  open: 'idle',
  waiting: 'waiting',
  idle: 'idle',
  stale: 'idle'
}

function getHighestPriorityAnimation(threads: ActiveThread[]): string {
  if (threads.length === 0) return 'idle'
  const priority = { error: 5, busy: 4, waiting: 3, open: 2, stale: 1, idle: 0 }
  const sorted = [...threads].sort((a, b) => priority[b.status] - priority[a.status])
  return ANIMATION_MAP[sorted[0].status] ?? 'idle'
}

function applyScale(scale: number) {
  currentScale = scale
  if (engine) engine.setScale(scale)
  reportBounds()
}

interface VirtualBounds { x: number; y: number; w: number; h: number }
let virtualBounds: VirtualBounds | null = null

function clampToVisible(x: number, y: number, container: HTMLElement) {
  // The overlay spans the union of every connected monitor; window.screen.*
  // only describes the primary one, so clamping against it would push valid
  // secondary-monitor positions back onto the primary screen. Use Rust's
  // virtual-desktop bounds if we have them, otherwise leave the position
  // untouched.
  const vb = virtualBounds
  if (!vb || vb.w <= 0 || vb.h <= 0) {
    return { x, y }
  }
  const petW = container.offsetWidth || 96
  const petH = container.offsetHeight || 100
  let newX = x, newY = y
  // If the pet would be entirely outside the union of monitors, reel it back
  // so it's at least partially visible. Don't touch positions inside the
  // virtual desktop, even if they're on a non-primary display.
  if (x + petW <= vb.x) newX = vb.x + 100
  else if (x >= vb.x + vb.w) newX = vb.x + vb.w - petW - 100
  if (y + petH <= vb.y) newY = vb.y + 100
  else if (y >= vb.y + vb.h) newY = vb.y + vb.h - petH - 100
  return { x: newX, y: newY }
}

function positionPet(x: number, y: number) {
  const container = document.getElementById('pet-container')!
  const clamped = clampToVisible(x, y, container)
  container.style.left = `${clamped.x}px`
  container.style.top = `${clamped.y}px`
  reportBounds()
  // No auto-save here. Only drag end and the menu/config flow mutate position.
}

function reportBounds() {
  // Report each interactive element as its own hit region. The Rust cursor
  // poller checks each rect individually, so transparent gaps between
  // sprite/card/chevron stay click-through to whatever's underneath the
  // overlay instead of getting captured by a union bounding box.
  const regions: { x: number; y: number; w: number; h: number }[] = []
  const collect = (id: string, visible?: () => boolean) => {
    const el = document.getElementById(id)
    if (!el) return
    if (visible && !visible()) return
    const r = el.getBoundingClientRect()
    if (r.width <= 0 || r.height <= 0) return
    regions.push({ x: r.left, y: r.top, w: r.width, h: r.height })
  }
  collect('pet-canvas')
  collect('thread-card', () => {
    const c = document.getElementById('thread-card')
    return !!c && c.style.display !== 'none'
  })
  collect('thread-chevron', () => {
    const c = document.getElementById('thread-chevron')
    return !!c && c.classList.contains('visible')
  })
  invoke('set_pet_hit_regions', { regions })
}

async function spritesheetUrl(pet: PetInfo): Promise<string> {
  // ID-based read, not path-based — the Rust side resolves the path itself
  // so the renderer can't request arbitrary files.
  const bytes = await invoke<number[]>('read_pet_image', { petId: pet.id })
  const u8 = new Uint8Array(bytes)
  const ext = pet.spritesheetAbsPath.split('.').pop()?.toLowerCase() ?? ''
  const mime = ext === 'webp' ? 'image/webp' : ext === 'png' ? 'image/png' : 'application/octet-stream'
  return URL.createObjectURL(new Blob([u8], { type: mime }))
}

function applyTextSize(size: string) {
  const root = document.documentElement
  const map: Record<string, { title: string; subtitle: string; pad: string; gap: string }> = {
    small:  { title: '10px',  subtitle: '8.5px', pad: '6px 10px',  gap: '10px' },
    medium: { title: '11px',  subtitle: '9.5px', pad: '8px 12px',  gap: '12px' },
    large:  { title: '13px',  subtitle: '11px',  pad: '10px 14px', gap: '14px' },
    xlarge: { title: '15px',  subtitle: '12.5px', pad: '12px 16px', gap: '16px' }
  }
  const v = map[size] ?? map.medium
  root.style.setProperty('--thread-title-size', v.title)
  root.style.setProperty('--thread-subtitle-size', v.subtitle)
  root.style.setProperty('--thread-card-pad', v.pad)
  root.style.setProperty('--thread-card-gap', v.gap)
}

function debugEnabled(): boolean {
  return (
    new URLSearchParams(location.search).has('debug') ||
    location.hash.includes('debug') ||
    !!(window as any).__PET_DEBUG__
  )
}

function debugLog(msg: string) {
  if (!debugEnabled()) return
  let el = document.getElementById('pet-debug-log')
  if (!el) {
    el = document.createElement('div')
    el.id = 'pet-debug-log'
    el.style.cssText = 'position:fixed;bottom:8px;right:8px;max-width:420px;max-height:30vh;overflow:auto;background:rgba(20,20,22,0.92);color:#cfc;padding:8px;border-radius:6px;font:11px/1.4 ui-monospace,monospace;z-index:99999;white-space:pre-wrap;pointer-events:none;'
    document.body.appendChild(el)
  }
  el.textContent += `[${Date.now() % 100000}] ${msg}\n`
}

async function init() {
  const container = document.getElementById('pet-container')!
  const canvas = document.getElementById('pet-canvas') as HTMLCanvasElement

  engine = new SpriteEngine(canvas, codexPetAtlas)
  threadLabel = new ThreadLabel(container, () => reportBounds())

  let dragOffsetX = 0
  let dragOffsetY = 0
  let lastDragX = 0
  let stillTimer: ReturnType<typeof setTimeout> | null = null
  let lastDragAnim = ''

  const setDragAnim = (name: string) => {
    if (lastDragAnim === name) return
    lastDragAnim = name
    engine?.setAnimation(name)
  }

  // mouseenter/leave handlers were the Electron approach (with forward:true).
  // Tauri uses cursor-position polling in Rust instead, so we don't toggle
  // here — that just races with the polling thread.

  container.addEventListener('pointerdown', (e: PointerEvent) => {
    // Block right-click and middle-click from starting a drag, but accept any
    // primary-button-ish event (left mouse, touch, pen). PointerEvent.button
    // can be -1 for some synthetic events; treating only 1/2 as "blocked"
    // keeps the common cases working.
    if (e.button === 1 || e.button === 2) return
    isDragging = true
    invoke('set_dragging', { dragging: true })
    const rect = container.getBoundingClientRect()
    dragOffsetX = e.clientX - rect.left
    dragOffsetY = e.clientY - rect.top
    lastDragX = e.clientX
    container.setPointerCapture(e.pointerId)
    invoke('set_ignore_mouse_events', { ignore: false })
    setDragAnim('jumping')
  })

  container.addEventListener('pointermove', (e: PointerEvent) => {
    if (!isDragging) return
    const x = e.clientX - dragOffsetX
    const y = e.clientY - dragOffsetY
    container.style.left = `${x}px`
    container.style.top = `${y}px`
    reportBounds()
    const dx = e.clientX - lastDragX
    if (Math.abs(dx) > 1) {
      setDragAnim(dx > 0 ? 'running-right' : 'running-left')
    }
    lastDragX = e.clientX
    if (stillTimer) clearTimeout(stillTimer)
    stillTimer = setTimeout(() => {
      if (isDragging) setDragAnim('jumping')
    }, 140)
  })

  const cancelDrag = (pointerId?: number) => {
    if (!isDragging) return
    isDragging = false
    invoke('set_dragging', { dragging: false })
    if (stillTimer) { clearTimeout(stillTimer); stillTimer = null }
    lastDragAnim = ''
    if (pointerId != null) {
      try { container.releasePointerCapture(pointerId) } catch {}
    }
    reportBounds()
    engine?.setAnimation(debugAnimation ?? lastThreadAnimation)
  }

  const endDrag = (e: PointerEvent) => {
    if (!isDragging) return
    const rect = container.getBoundingClientRect()
    invoke('save_position', { position: { x: Math.round(rect.left), y: Math.round(rect.top) } })
    invoke('trigger_poll')
    cancelDrag(e.pointerId)
  }

  container.addEventListener('pointerup', endDrag)
  container.addEventListener('pointercancel', endDrag)
  // If the cursor leaves the window entirely (e.g. the user drags off-screen),
  // any pointerup that lands elsewhere never reaches us. Catch that case here.
  window.addEventListener('blur', () => cancelDrag())

  container.addEventListener('contextmenu', (e: MouseEvent) => {
    e.preventDefault()
    // Right-click should never leave the pet stuck mid-drag — abort any
    // in-progress drag before showing the menu.
    cancelDrag()
    invoke('show_context_menu')
  })

  container.addEventListener('wheel', (e: WheelEvent) => {
    e.preventDefault()
    const delta = e.deltaY > 0 ? -0.1 : 0.1
    const newScale = Math.max(0.25, Math.min(3, currentScale + delta))
    if (newScale !== currentScale) {
      applyScale(newScale)
      invoke('save_scale', { scale: newScale })
    }
  }, { passive: false })

  // Subscribe to backend events
  await listen<PetInfo>('pet-data', async (event) => {
    if (engine) {
      const src = await spritesheetUrl(event.payload)
      await engine.loadSpritesheet(src)
      engine.setAnimation('idle')
      engine.start()
    }
  })

  await listen<ActiveThread[]>('thread-state', (event) => {
    const threads = event.payload
    lastThreadAnimation = getHighestPriorityAnimation(threads)
    if (engine && !debugAnimation && !isDragging) {
      engine.setAnimation(lastThreadAnimation)
    }
    threadLabel?.update(threads)
  })

  await listen<AppConfig>('config', (event) => {
    const cfg = event.payload
    applyScale(cfg.scale ?? 1)
    if (cfg.position) positionPet(cfg.position.x, cfg.position.y)
    applyTextSize(cfg.textSize ?? 'medium')
    if (engine && cfg.animationSpeeds) {
      for (const [n, s] of Object.entries(cfg.animationSpeeds)) {
        engine.setAnimationSpeed(n, s as number)
      }
    }
  })

  await listen<string | null>('debug-animation', (event) => {
    debugAnimation = event.payload
    if (engine && !isDragging) {
      engine.setAnimation(event.payload ?? lastThreadAnimation)
    }
  })

  await listen<{ name: string; enabled: boolean }>('pingpong-override', (event) => {
    const anim = codexPetAtlas.animations[event.payload.name]
    if (anim) anim.pingpong = event.payload.enabled
  })

  await listen<{ name: string; speed: number }>('animation-speed', (event) => {
    engine?.setAnimationSpeed(event.payload.name, event.payload.speed)
  })

  // Pull virtual-desktop bounds before applying the saved position so the
  // first clamp pass uses real geometry instead of window.screen.* (which
  // would be primary-monitor-only).
  try {
    virtualBounds = await invoke<VirtualBounds>('get_virtual_bounds')
  } catch (e) {
    console.error('get_virtual_bounds failed:', e)
  }

  // Bootstrap from invoke after listeners are wired, since events emitted
  // during the Rust setup() phase fire before the JS side can attach.
  try {
    const cfg = await invoke<AppConfig>('get_config')
    applyScale(cfg.scale ?? 1)
    if (cfg.position) positionPet(cfg.position.x, cfg.position.y)
    applyTextSize(cfg.textSize ?? 'medium')
  } catch (e) {
    console.error('get_config failed:', e)
  }

  try {
    const pets = await invoke<PetInfo[]>('list_pets')
    if (pets.length > 0 && engine) {
      const cfg = await invoke<AppConfig>('get_config')
      const selected =
        (cfg.selectedPetId && pets.find(p => p.id === cfg.selectedPetId)) || pets[0]
      const src = await spritesheetUrl(selected)
      await engine.loadSpritesheet(src)
      engine.setAnimation('idle')
      engine.start()
    }
  } catch (e) {
    console.error('pet bootstrap failed:', e)
  }

  // The first poll fires inside Rust setup() before any of the listeners
  // above have attached, so the resulting thread-state is lost. Now that the
  // renderer is fully wired, ask Rust for a fresh poll so an already-running
  // session shows up immediately instead of after the next 30s tick.
  invoke('trigger_poll')
}

init().catch(console.error)
