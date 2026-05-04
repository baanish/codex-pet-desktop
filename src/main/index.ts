import { app, BrowserWindow, ipcMain, Menu, MenuItemConstructorOptions } from 'electron'
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

function buildContextMenu(): Menu {
  const cfg = config!.get()
  const currentScale = cfg.scale ?? 1
  const currentPoll = cfg.pollIntervalMs ?? 30000

  const scaleOptions: MenuItemConstructorOptions[] = [
    { label: '50%', type: 'radio', checked: currentScale === 0.5, click: () => setScale(0.5) },
    { label: '75%', type: 'radio', checked: currentScale === 0.75, click: () => setScale(0.75) },
    { label: '100%', type: 'radio', checked: currentScale === 1, click: () => setScale(1) },
    { label: '150%', type: 'radio', checked: currentScale === 1.5, click: () => setScale(1.5) },
    { label: '200%', type: 'radio', checked: currentScale === 2, click: () => setScale(2) },
    { label: '300%', type: 'radio', checked: currentScale === 3, click: () => setScale(3) },
  ]

  const pollOptions: MenuItemConstructorOptions[] = [
    { label: '10s', type: 'radio', checked: currentPoll === 10000, click: () => setPoll(10000) },
    { label: '30s', type: 'radio', checked: currentPoll === 30000, click: () => setPoll(30000) },
    { label: '60s', type: 'radio', checked: currentPoll === 60000, click: () => setPoll(60000) },
    { label: '120s', type: 'radio', checked: currentPoll === 120000, click: () => setPoll(120000) },
  ]

  return Menu.buildFromTemplate([
    { label: 'Size', submenu: scaleOptions },
    { label: 'Poll Interval', submenu: pollOptions },
    { type: 'separator' },
    {
      label: 'Always on Top',
      type: 'checkbox',
      checked: cfg.alwaysOnTop,
      click: (menuItem) => {
        config!.update({ alwaysOnTop: menuItem.checked })
        if (mainWindow && !mainWindow.isDestroyed()) {
          mainWindow.setAlwaysOnTop(menuItem.checked)
        }
      }
    },
    { type: 'separator' },
    {
      label: 'Quit',
      accelerator: 'CmdOrCtrl+Q',
      click: () => app.quit()
    }
  ])
}

function setScale(scale: number) {
  config!.update({ scale })
  if (mainWindow && !mainWindow.isDestroyed()) {
    const { width, height } = getWindowSize(scale)
    const [x, y] = mainWindow.getPosition()
    mainWindow.setBounds({ x, y, width, height })
    sendToRenderer('config', config!.get())
  }
}

function setPoll(ms: number) {
  config!.update({ pollIntervalMs: ms })
  monitor?.destroy()
  monitor!.start(ms)
}

app.whenReady().then(() => {
  config = new ConfigStore(app.getPath('userData'))

  const adapters = [
    new OpenCodeAdapter(),
    new ClaudeCodeAdapter(),
    new CodexAdapter()
  ]

  monitor = new ThreadMonitor(adapters, (threads: ActiveThread[]) => {
    console.log(`[main] sending ${threads.length} threads:`, threads.map(t => `${t.tool}:${t.status}:${t.title}`).join(', '))
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

  ipcMain.on('set-window-position', (_event, x: unknown, y: unknown) => {
    if (mainWindow && !mainWindow.isDestroyed()) {
      const nx = Number(x)
      const ny = Number(y)
      if (Number.isFinite(nx) && Number.isFinite(ny)) {
        mainWindow.setPosition(Math.round(nx), Math.round(ny))
      }
    }
  })

  ipcMain.on('set-window-size', (_event, width: unknown, height: unknown) => {
    if (mainWindow && !mainWindow.isDestroyed()) {
      const nw = Number(width)
      const nh = Number(height)
      if (Number.isFinite(nw) && Number.isFinite(nh) && nw > 0 && nh > 0) {
        const [currentX, currentY] = mainWindow.getPosition()
        mainWindow.setBounds({
          x: currentX,
          y: currentY,
          width: Math.round(nw),
          height: Math.round(nh)
        })
      }
    }
  })

  ipcMain.on('save-scale', (_event, scale: unknown) => {
    const ns = Number(scale)
    if (Number.isFinite(ns) && ns > 0) {
      config!.update({ scale: ns })
    }
  })

  ipcMain.on('show-context-menu', () => {
    const menu = buildContextMenu()
    menu.popup({ window: mainWindow! })
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
