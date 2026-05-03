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
  triggerPoll: () => {
    ipcRenderer.send('trigger-poll')
  }
})
