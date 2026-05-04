import { ThreadAdapter } from './adapter'
import { ActiveThread } from '../../shared/types'

export class ThreadMonitor {
  private adapters: ThreadAdapter[]
  private interval: ReturnType<typeof setInterval> | null = null
  private onState: (threads: ActiveThread[]) => void

  constructor(adapters: ThreadAdapter[], onState: (threads: ActiveThread[]) => void) {
    this.adapters = adapters.filter(a => a.isInstalled())
    this.onState = onState
  }

  start(intervalMs: number) {
    const poll = async () => {
      try {
        const results = await Promise.allSettled(
          this.adapters.map(a => a.poll())
        )
        const threads = results
          .filter((r): r is PromiseFulfilledResult<ActiveThread[]> => r.status === 'fulfilled')
          .flatMap(r => r.value)
        this.onState(threads)
      } catch {
        // Don't crash on poll errors
      }
    }

    this.interval = setInterval(poll, intervalMs)
    poll() // initial poll
  }

  triggerPoll() {
    const poll = async () => {
      try {
        const results = await Promise.allSettled(
          this.adapters.map(a => a.poll())
        )
        const threads = results
          .filter((r): r is PromiseFulfilledResult<ActiveThread[]> => r.status === 'fulfilled')
          .flatMap(r => r.value)
        this.onState(threads)
      } catch {
        // Don't crash on poll errors
      }
    }
    poll()
  }

  destroy() {
    if (this.interval) {
      clearInterval(this.interval)
      this.interval = null
    }
    this.adapters.forEach(a => a.destroy())
  }
}
