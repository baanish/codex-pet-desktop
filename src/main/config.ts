import { existsSync, readFileSync, writeFileSync, mkdirSync } from 'fs'
import { dirname, join } from 'path'
import { AppConfig, DEFAULT_CONFIG } from '../shared/types'

export class ConfigStore {
  private path: string
  private config: AppConfig
  private writeTimeout: ReturnType<typeof setTimeout> | null = null

  constructor(userDataPath: string) {
    this.path = join(userDataPath, 'config.json')
    this.config = this.load()
  }

  private load(): AppConfig {
    try {
      if (existsSync(this.path)) {
        const raw = readFileSync(this.path, 'utf-8')
        const parsed = JSON.parse(raw)
        return { ...DEFAULT_CONFIG, ...parsed }
      }
    } catch {
      // Corrupted config, use defaults
    }
    return { ...DEFAULT_CONFIG }
  }

  get(): AppConfig {
    return { ...this.config }
  }

  update(partial: Partial<AppConfig>) {
    this.config = { ...this.config, ...partial }
    this.scheduleWrite()
  }

  private scheduleWrite() {
    if (this.writeTimeout) {
      clearTimeout(this.writeTimeout)
    }
    this.writeTimeout = setTimeout(() => {
      this.flush()
    }, 500)
  }

  flush() {
    if (this.writeTimeout) {
      clearTimeout(this.writeTimeout)
      this.writeTimeout = null
    }
    try {
      const dir = dirname(this.path)
      if (!existsSync(dir)) {
        mkdirSync(dir, { recursive: true })
      }
      writeFileSync(this.path, JSON.stringify(this.config, null, 2), 'utf-8')
    } catch {
      // Don't crash on write errors
    }
  }
}
