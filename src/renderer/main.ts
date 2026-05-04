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
      setWindowPosition: (x: number, y: number) => void
      saveScale: (scale: number) => void
      setWindowSize: (width: number, height: number) => void
      showContextMenu: () => void
      triggerPoll: () => void
    }
  }
}

let engine: SpriteEngine | null = null
let threadLabel: ThreadLabel | null = null
let dragHandler: DragHandler | null = null
let currentScale = 1

const ANIMATION_MAP: Record<string, string> = {
  error: 'failed',
  busy: 'running',
  waiting: 'waiting',
  idle: 'idle',
  stale: 'idle'
}

const BASE_CELL_WIDTH = 192
const BASE_CELL_HEIGHT = 208
const LABEL_HEIGHT = 50

function getHighestPriorityAnimation(threads: ActiveThread[]): string {
  if (threads.length === 0) return 'idle'

  const priority = { error: 4, busy: 3, waiting: 2, stale: 1, idle: 0 }
  const sorted = [...threads].sort((a, b) => priority[b.status] - priority[a.status])
  return ANIMATION_MAP[sorted[0].status] ?? 'idle'
}

function applyScale(scale: number) {
  currentScale = scale

  const container = document.getElementById('pet-container')!
  const canvas = document.getElementById('pet-canvas') as HTMLCanvasElement

  const w = Math.round(BASE_CELL_WIDTH * scale)
  const h = Math.round(BASE_CELL_HEIGHT * scale)
  const labelH = Math.round(LABEL_HEIGHT * scale)

  container.style.width = `${w}px`
  container.style.height = `${h + labelH}px`

  canvas.style.width = `${w}px`
  canvas.style.height = `${h}px`

  if (engine) {
    engine.setScale(scale)
  }

  window.petBridge.setWindowSize(w, h + labelH)
}

async function init() {
  const container = document.getElementById('pet-container')!
  const canvas = document.getElementById('pet-canvas') as HTMLCanvasElement

  engine = new SpriteEngine(canvas, codexPetAtlas)
  threadLabel = new ThreadLabel(container)

  dragHandler = new DragHandler(container, () => {
    window.petBridge.triggerPoll()
  })

  // Right-click context menu
  container.addEventListener('contextmenu', (e: MouseEvent) => {
    e.preventDefault()
    window.petBridge.showContextMenu()
  })

  // Scroll wheel to resize
  container.addEventListener('wheel', (e: WheelEvent) => {
    e.preventDefault()
    const delta = e.deltaY > 0 ? -0.1 : 0.1
    const newScale = Math.max(0.5, Math.min(3, currentScale + delta))
    if (newScale !== currentScale) {
      applyScale(newScale)
      window.petBridge.saveScale(newScale)
    }
  }, { passive: false })

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
    applyScale(config.scale ?? 1)
  })
}

init().catch(console.error)
