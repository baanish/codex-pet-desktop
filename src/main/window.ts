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

  return win
}
