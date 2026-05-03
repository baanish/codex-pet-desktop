import { execSync } from 'child_process'
import { ActiveThread } from '../../shared/types'

export interface ThreadAdapter {
  readonly id: string
  readonly displayName: string
  isInstalled(): boolean
  poll(): Promise<ActiveThread[]>
  destroy(): void
}

export function isPidAlive(pid: number): boolean {
  try {
    process.kill(pid, 0)
    return true
  } catch {
    return false
  }
}

export function findProcessByName(pattern: string): number | null {
  try {
    const result = execSync(`pgrep -f "${pattern}"`, {
      encoding: 'utf-8',
      timeout: 5000,
      stdio: ['pipe', 'pipe', 'pipe']
    })
    const pids = result.trim().split('\n').filter(Boolean)
    if (pids.length > 0) {
      return parseInt(pids[0], 10)
    }
  } catch {
    // pgrep returns exit 1 if no match
  }
  return null
}

export function isLockHeldByProcess(lockPath: string): boolean {
  try {
    const result = execSync(`lsof "${lockPath}"`, {
      encoding: 'utf-8',
      timeout: 5000,
      stdio: ['pipe', 'pipe', 'pipe']
    })
    return result.trim().length > 0
  } catch {
    return false
  }
}
