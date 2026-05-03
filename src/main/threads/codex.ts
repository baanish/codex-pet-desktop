import { existsSync, readdirSync } from 'fs'
import { join } from 'path'
import { homedir } from 'os'
import { ThreadAdapter, isLockHeldByProcess, findProcessByName } from './adapter'
import { ActiveThread } from '../../shared/types'

export class CodexAdapter implements ThreadAdapter {
  readonly id = 'codex'
  readonly displayName = 'Codex'

  private codexDir: string
  private dbPath: string

  constructor() {
    this.codexDir = join(homedir(), '.codex')
    this.dbPath = join(this.codexDir, 'state_5.sqlite')
  }

  isInstalled(): boolean {
    return existsSync(this.codexDir)
  }

  async poll(): Promise<ActiveThread[]> {
    try {
      const locksDir = join(this.codexDir, 'tmp/arg0')
      if (!existsSync(locksDir)) return []

      const lockDirs = readdirSync(locksDir).filter(d => d.startsWith('codex-'))
      let hasLiveLock = false

      for (const dir of lockDirs) {
        const lockFile = join(locksDir, dir, '.lock')
        if (existsSync(lockFile)) {
          if (isLockHeldByProcess(lockFile)) {
            hasLiveLock = true
            break
          }
        }
      }

      if (!hasLiveLock) {
        const pid = findProcessByName('codex')
        if (!pid) return []
        hasLiveLock = true
      }

      if (!existsSync(this.dbPath)) {
        return [{ tool: this.id, status: 'busy', title: null }]
      }

      const Database = require('better-sqlite3')
      const db = new Database(this.dbPath, {
        readonly: true,
        fileMustExist: true
      })

      try {
        const row = db.prepare(
          'SELECT title FROM threads WHERE archived=0 ORDER BY updated_at DESC LIMIT 1'
        ).get() as { title: string | null } | undefined

        return [{
          tool: this.id,
          status: 'busy',
          title: row?.title ?? null
        }]
      } finally {
        db.close()
      }
    } catch {
      return []
    }
  }

  destroy(): void {}
}
