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
