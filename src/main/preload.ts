import { contextBridge, ipcRenderer } from 'electron'

contextBridge.exposeInMainWorld('petBridge', {
  onThreadState: (callback: (data: any) => void) => {
    ipcRenderer.on('thread-state', (_event, data) => callback(data))
  },
  onPetData: (callback: (data: any) => void) => {
    ipcRenderer.on('pet-data', (_event, data) => callback(data))
  },
  onConfig: (callback: (data: any) => void) => {
    ipcRenderer.on('config', (_event, data) => callback(data))
  },
  savePosition: (position: { x: number; y: number }) => {
    ipcRenderer.send('save-position', position)
  },
  setWindowPosition: (x: number, y: number) => {
    ipcRenderer.send('set-window-position', x, y)
  },
  saveScale: (scale: number) => {
    ipcRenderer.send('save-scale', scale)
  },
  setWindowSize: (width: number, height: number) => {
    ipcRenderer.send('set-window-size', width, height)
  },
  triggerPoll: () => {
    ipcRenderer.send('trigger-poll')
  }
})
