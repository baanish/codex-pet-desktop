export type ThreadStatus = 'busy' | 'idle' | 'waiting' | 'error' | 'stale'

export interface ActiveThread {
  tool: string
  status: ThreadStatus
  title: string | null
}
