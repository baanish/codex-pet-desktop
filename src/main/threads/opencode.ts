import { existsSync, statSync } from 'fs'
import { join } from 'path'
import { homedir } from 'os'
import { ThreadAdapter, findProcessByName } from './adapter'
import { ActiveThread } from '../../shared/types'

export class OpenCodeAdapter implements ThreadAdapter {
  readonly id = 'opencode'
  readonly displayName = 'OpenCode'

  private dbPath: string
  private walPath: string

  constructor() {
    const dataDir = join(homedir(), '.local/share/opencode')
    this.dbPath = join(dataDir, 'opencode.db')
    this.walPath = join(dataDir, 'opencode.db-wal')
  }

  isInstalled(): boolean {
    return existsSync(this.dbPath)
  }

  async poll(): Promise<ActiveThread[]> {
    try {
      if (!existsSync(this.walPath)) return []
      const walStat = statSync(this.walPath)
      if (walStat.size === 0) return []

      const pid = findProcessByName('opencode')
      if (!pid) {
        return [{ tool: this.id, status: 'stale', title: null }]
      }

      const Database = require('better-sqlite3')
      const db = new Database(this.dbPath, {
        readonly: true,
        fileMustExist: true
      })

      try {
        const row = db.prepare(
          'SELECT title FROM session WHERE time_archived IS NULL ORDER BY time_updated DESC LIMIT 1'
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
