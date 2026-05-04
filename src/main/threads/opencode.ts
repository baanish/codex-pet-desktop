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
    const installed = existsSync(this.dbPath)
    console.log(`[OpenCode] isInstalled: ${installed} (db: ${this.dbPath})`)
    return installed
  }

  async poll(): Promise<ActiveThread[]> {
    try {
      if (!existsSync(this.walPath)) {
        console.log('[OpenCode] No WAL file')
        return []
      }
      const walStat = statSync(this.walPath)
      if (walStat.size === 0) {
        console.log('[OpenCode] WAL empty')
        return []
      }

      const pid = findProcessByName('opencode')
      console.log(`[OpenCode] Process found: ${pid}`)
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

        console.log(`[OpenCode] DB query result:`, row)
        return [{
          tool: this.id,
          status: 'busy',
          title: row?.title ?? null
        }]
      } finally {
        db.close()
      }
    } catch (err) {
      console.error('[OpenCode] Poll error:', err)
      return []
    }
  }

  destroy(): void {}
}
