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
    const installed = existsSync(join(homedir(), '.claude'))
    console.log(`[ClaudeCode] isInstalled: ${installed}`)
    return installed
  }

  async poll(): Promise<ActiveThread[]> {
    if (!existsSync(this.sessionsDir)) {
      console.log('[ClaudeCode] No sessions dir')
      return []
    }

    try {
      const files = readdirSync(this.sessionsDir).filter(f => f.endsWith('.json'))
      console.log(`[ClaudeCode] Found ${files.length} session files: ${files.join(', ')}`)
      const active: ActiveThread[] = []

      for (const file of files) {
        const filePath = join(this.sessionsDir, file)
        try {
          const data: ClaudeSessionFile = JSON.parse(readFileSync(filePath, 'utf-8'))
          console.log(`[ClaudeCode] ${file}: pid=${data.pid}, status=${data.status}, name=${data.name}`)

          if (data.pid && isPidAlive(data.pid)) {
            console.log(`[ClaudeCode] PID ${data.pid} is alive`)
            active.push({
              tool: this.id,
              status: data.status === 'busy' ? 'busy' : 'idle',
              title: data.name ?? null
            })
            continue
          }

          const mtime = statSync(filePath).mtimeMs
          const age = Date.now() - mtime
          console.log(`[ClaudeCode] ${file}: pid ${data.pid} not alive, mtime age: ${age}ms`)
          if (age < 60_000) {
            active.push({
              tool: this.id,
              status: 'stale',
              title: data.name ?? null
            })
          }
        } catch (err) {
          console.error(`[ClaudeCode] Error reading ${file}:`, err)
        }
      }

      return active
    } catch (err) {
      console.error('[ClaudeCode] Poll error:', err)
      return []
    }
  }

  destroy(): void {}
}
