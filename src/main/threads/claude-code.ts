import { existsSync, readdirSync, readFileSync, statSync } from 'fs'
import { join } from 'path'
import { homedir } from 'os'
import { ThreadAdapter, isPidAlive } from './adapter'
import { ActiveThread } from '../../shared/types'

interface ClaudeSessionFile {
  pid?: number
  name?: string
  status?: string
  updatedAt?: number
}

export class ClaudeCodeAdapter implements ThreadAdapter {
  readonly id = 'claude-code'
  readonly displayName = 'Claude Code'

  private sessionsDir: string

  constructor() {
    this.sessionsDir = join(homedir(), '.claude/sessions')
  }

  isInstalled(): boolean {
    return existsSync(join(homedir(), '.claude'))
  }

  async poll(): Promise<ActiveThread[]> {
    if (!existsSync(this.sessionsDir)) return []

    try {
      const files = readdirSync(this.sessionsDir).filter(f => f.endsWith('.json'))
      const active: ActiveThread[] = []

      for (const file of files) {
        const filePath = join(this.sessionsDir, file)
        try {
          const data: ClaudeSessionFile = JSON.parse(readFileSync(filePath, 'utf-8'))

          if (data.pid && isPidAlive(data.pid)) {
            active.push({
              tool: this.id,
              status: data.status === 'busy' ? 'busy' : 'idle',
              title: data.name ?? null
            })
            continue
          }

          const mtime = statSync(filePath).mtimeMs
          if (Date.now() - mtime < 60_000) {
            active.push({
              tool: this.id,
              status: 'stale',
              title: data.name ?? null
            })
          }
        } catch {
          // Skip malformed files
        }
      }

      return active
    } catch {
      return []
    }
  }

  destroy(): void {}
}
