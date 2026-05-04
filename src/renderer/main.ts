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

function applyScale(scale: number) {
  currentScale = scale
  if (engine) engine.setScale(scale)
  reportBounds()
}

function clampToVisible(x: number, y: number, container: HTMLElement) {
  // Only clamp if the renderer can confidently reason about screen geometry.
  // In some Tauri/WebKit configurations window.screenX/Y come back unreliable
  // (e.g., the screen height instead of the window's top), which would make
  // every saved position get pushed off-screen.
  const winX = window.screenX
  const winY = window.screenY
  const sw = window.screen.width
  const sh = window.screen.height
  if (
    !Number.isFinite(winX) || !Number.isFinite(winY) ||
    winX < -1000 || winX >= sw ||
    winY < -1000 || winY >= sh
  ) {
    return { x, y }
  }
  const petW = container.offsetWidth || 96
  const petH = container.offsetHeight || 100
  const petScreenLeft = winX + x
  const petScreenRight = petScreenLeft + petW
  const petScreenTop = winY + y
  const petScreenBottom = petScreenTop + petH
  let newX = x, newY = y
  if (petScreenRight < 0) newX = -winX + 100
  else if (petScreenLeft >= sw) newX = -winX + sw - petW - 100
  if (petScreenBottom < 0) newY = -winY + 100
  else if (petScreenTop >= sh) newY = -winY + sh - petH - 100
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
  const container = document.getElementById('pet-container')
  if (!container) return
  // Include the absolute-positioned card too, since the card is also clickable.
  const containerRect = container.getBoundingClientRect()
  const card = document.getElementById('thread-card')
  let left = containerRect.left
  let top = containerRect.top
  let right = containerRect.right
  let bottom = containerRect.bottom
  if (card && card.style.display !== 'none') {
    const cr = card.getBoundingClientRect()
    if (cr.width > 0 && cr.height > 0) {
      left = Math.min(left, cr.left)
      top = Math.min(top, cr.top)
      right = Math.max(right, cr.right)
      bottom = Math.max(bottom, cr.bottom)
    }
  }
  const chev = document.getElementById('thread-chevron')
  if (chev && chev.classList.contains('visible')) {
    const cr = chev.getBoundingClientRect()
    if (cr.width > 0 && cr.height > 0) {
      left = Math.min(left, cr.left)
      top = Math.min(top, cr.top)
      right = Math.max(right, cr.right)
      bottom = Math.max(bottom, cr.bottom)
    }
  }
  invoke('set_pet_bounds', {
    x: left,
    y: top,
    w: right - left,
    h: bottom - top
  })
}

async function spritesheetUrl(absPath: string): Promise<string> {
  // Tauri's asset:// protocol misbehaved with absolute paths in this scope,
  // so we marshal the bytes back through IPC and use a blob URL instead.
  const bytes = await invoke<number[]>('read_pet_image', { path: absPath })
  const u8 = new Uint8Array(bytes)
  const ext = absPath.split('.').pop()?.toLowerCase() ?? ''
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
    const newScale = Math.max(0.5, Math.min(3, currentScale + delta))
    if (newScale !== currentScale) {
      applyScale(newScale)
      invoke('save_scale', { scale: newScale })
    }
  }, { passive: false })

  // Subscribe to backend events
  await listen<PetInfo>('pet-data', async (event) => {
    if (engine) {
      const src = await spritesheetUrl(event.payload.spritesheetAbsPath)
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
      const src = await spritesheetUrl(selected.spritesheetAbsPath)
      await engine.loadSpritesheet(src)
      engine.setAnimation('idle')
      engine.start()
    }
  } catch (e) {
    console.error('pet bootstrap failed:', e)
  }
}

init().catch(console.error)
