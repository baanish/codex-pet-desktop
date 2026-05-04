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
      frameDurations: [420, 180, 180, 220, 220, 480],
      pingpong: true
    },
    'running-right': {
      row: 1,
      frames: 8,
      frameDurations: [180, 180, 180, 180, 180, 180, 180, 280],
      pingpong: true
    },
    'running-left': {
      row: 2,
      frames: 8,
      frameDurations: [180, 180, 180, 180, 180, 180, 180, 280],
      pingpong: true
    },
    waving: {
      row: 3,
      frames: 4,
      frameDurations: [220, 220, 220, 380]
    },
    jumping: {
      row: 4,
      frames: 5,
      frameDurations: [200, 200, 200, 200, 360]
    },
    failed: {
      row: 5,
      frames: 8,
      frameDurations: [200, 200, 200, 200, 200, 200, 200, 320]
    },
    waiting: {
      row: 6,
      frames: 6,
      frameDurations: [220, 220, 220, 220, 220, 360],
      pingpong: true
    },
    running: {
      row: 7,
      frames: 6,
      frameDurations: [180, 180, 180, 180, 180, 280]
    },
    review: {
      row: 8,
      frames: 6,
      frameDurations: [220, 220, 220, 220, 220, 360],
      pingpong: true
    }
  }
}
