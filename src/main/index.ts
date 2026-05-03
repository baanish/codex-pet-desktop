import { app, BrowserWindow, ipcMain } from 'electron'
import { join } from 'path'
import { createPetWindow, getWindowSize } from './window'
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
    const pets = loadPets()
    if (pets.length > 0) {
      const selected = pets.find(p => p.id === appConfig.selectedPetId) ?? pets[0]
      sendToRenderer('pet-data', selected)
    }

    sendToRenderer('config', appConfig)

    monitor!.start(appConfig.pollIntervalMs)
  })

  ipcMain.on('save-position', (_event, position: { x: number; y: number }) => {
    config!.update({ position })
  })

  ipcMain.on('set-window-position', (_event, x: number, y: number) => {
    if (mainWindow && !mainWindow.isDestroyed()) {
      mainWindow.setPosition(Math.round(x), Math.round(y))
    }
  })

  ipcMain.on('set-window-size', (_event, width: number, height: number) => {
    if (mainWindow && !mainWindow.isDestroyed()) {
      const [currentX, currentY] = mainWindow.getPosition()
      mainWindow.setBounds({
        x: currentX,
        y: currentY,
        width: Math.round(width),
        height: Math.round(height)
      })
    }
  })

  ipcMain.on('save-scale', (_event, scale: number) => {
    config!.update({ scale })
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
