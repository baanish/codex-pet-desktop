import { BrowserWindow, screen } from 'electron'
import { join } from 'path'
import { AppConfig } from '../shared/types'

export const BASE_CELL_WIDTH = 192
export const BASE_CELL_HEIGHT = 208
export const LABEL_HEIGHT = 50

export function getWindowSize(scale: number) {
  return {
    width: Math.round(BASE_CELL_WIDTH * scale),
    height: Math.round((BASE_CELL_HEIGHT + LABEL_HEIGHT) * scale)
  }
}

export function createPetWindow(config: AppConfig): BrowserWindow {
  const { position, alwaysOnTop, scale } = config
  const { width, height } = getWindowSize(scale)

  // Use full display bounds (not workArea) so the pet can sit under the dock/menu bar
  const display = screen.getPrimaryDisplay()
  const { x: screenX, y: screenY, width: screenW, height: screenH } = display.bounds
  const x = Math.max(screenX - width + 40, Math.min(position.x, screenX + screenW - 40))
  const y = Math.max(screenY - height + 40, Math.min(position.y, screenY + screenH - 40))

  const win = new BrowserWindow({
    width,
    height,
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

  return win
}
