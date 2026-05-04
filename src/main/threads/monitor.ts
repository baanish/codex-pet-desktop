import { ThreadAdapter } from './adapter'
import { ActiveThread } from '../../shared/types'

export class ThreadMonitor {
  private adapters: ThreadAdapter[]
  private interval: ReturnType<typeof setInterval> | null = null
  private onState: (threads: ActiveThread[]) => void

  constructor(adapters: ThreadAdapter[], onState: (threads: ActiveThread[]) => void) {
    this.adapters = adapters.filter(a => a.isInstalled())
    this.onState = onState
    console.log(`[ThreadMonitor] Active adapters: ${this.adapters.map(a => a.id).join(', ')}`)
  }

  start(intervalMs: number) {
    const poll = async () => {
      try {
        const results = await Promise.allSettled(
          this.adapters.map(a => a.poll())
        )

        for (let i = 0; i < results.length; i++) {
          const r = results[i]
          const adapter = this.adapters[i]
          if (r.status === 'rejected') {
            console.error(`[ThreadMonitor] ${adapter.id} poll rejected:`, r.reason)
          }
        }

        const threads = results
          .filter((r): r is PromiseFulfilledResult<ActiveThread[]> => r.status === 'fulfilled')
          .flatMap(r => r.value)

        if (threads.length > 0) {
          console.log(`[ThreadMonitor] Detected:`, threads.map(t => `${t.tool}:${t.status}:${t.title}`).join(', '))
        }

        this.onState(threads)
      } catch (err) {
        console.error('[ThreadMonitor] Poll error:', err)
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

        for (let i = 0; i < results.length; i++) {
          const r = results[i]
          const adapter = this.adapters[i]
          if (r.status === 'rejected') {
            console.error(`[ThreadMonitor] ${adapter.id} triggerPoll rejected:`, r.reason)
          }
        }

        const threads = results
          .filter((r): r is PromiseFulfilledResult<ActiveThread[]> => r.status === 'fulfilled')
          .flatMap(r => r.value)

        if (threads.length > 0) {
          console.log(`[ThreadMonitor] TriggerPoll detected:`, threads.map(t => `${t.tool}:${t.status}:${t.title}`).join(', '))
        }

        this.onState(threads)
      } catch (err) {
        console.error('[ThreadMonitor] triggerPoll error:', err)
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
