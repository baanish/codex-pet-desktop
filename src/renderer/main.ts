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
  })
}

init().catch(console.error)
