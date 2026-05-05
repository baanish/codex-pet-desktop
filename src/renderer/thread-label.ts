import { invoke } from '@tauri-apps/api/core'
import { ActiveThread, ThreadStatus } from '../shared/types'

const STATUS_PRIORITY: Record<ThreadStatus, number> = {
  error: 4,
  busy: 3,
  waiting: 2,
  stale: 1,
  idle: 0
}

const STATUS_WORD: Record<ThreadStatus, string> = {
  busy: 'Thinking',
  waiting: 'Waiting for input',
  error: 'Error',
  idle: 'Idle',
  stale: 'Idle'
}

export class ThreadLabel {
  private cardEl: HTMLElement
  private chevronEl: HTMLElement
  private chevronGlyphEl: HTMLElement
  private expanded = false
  private threads: ActiveThread[] = []
  private onResize: (() => void) | null
  /// Per-thread "details revealed" state, keyed by `tool|pid|cwd|title`. Click
  /// toggles. Resets when a thread disappears from the next poll.
  private revealed = new Set<string>()

  constructor(_container: HTMLElement, onResize?: () => void) {
    this.onResize = onResize ?? null
    this.cardEl = document.getElementById('thread-card')!
    this.chevronEl = document.getElementById('thread-chevron')!
    this.chevronGlyphEl = document.getElementById('chevron-glyph')!

    // Block pointerdown from bubbling so the container's drag handler doesn't
    // setPointerCapture and steal the chevron's click event.
    this.chevronEl.addEventListener('pointerdown', (e) => {
      e.stopPropagation()
    })
    this.chevronEl.addEventListener('click', (e) => {
      e.stopPropagation()
      this.expanded = !this.expanded
      this.render()
      this.onResize?.()
    })

    // Click a row → toggle the cwd / pid detail line. On reveal, also copy
    // the PID to the clipboard and flash a small toast as confirmation.
    // pointerdown is stopped so the container's drag handler doesn't
    // intercept the click.
    this.cardEl.addEventListener('pointerdown', (e) => {
      e.stopPropagation()
    })
    this.cardEl.addEventListener('click', (e) => {
      e.stopPropagation()
      const target = e.target as HTMLElement | null
      const dismiss = target?.closest('.thread-dismiss') as HTMLElement | null
      if (dismiss) {
        const key = dismiss.dataset.key
        const thread = this.threads.find(t => this.rowKey(t) === key)
        if (thread && this.isDismissible(thread)) {
          this.threads = this.threads.filter(t => this.rowKey(t) !== key)
          this.revealed.delete(key!)
          invoke('dismiss_thread', { thread }).catch(() => {})
          this.render()
          this.onResize?.()
        }
        return
      }
      const row = target?.closest('.thread-row') as HTMLElement | null
      if (!row) return
      const key = row.dataset.key
      if (!key) return
      const pid = row.dataset.pid
      if (this.revealed.has(key)) {
        this.revealed.delete(key)
      } else {
        this.revealed.add(key)
        if (pid) {
          navigator.clipboard?.writeText(pid).catch(() => {})
          this.flashCopied(row, `pid ${pid} copied`)
        }
      }
      this.render()
      this.onResize?.()
    })
  }

  private flashCopied(row: HTMLElement, message: string) {
    const prev = row.querySelector('.thread-toast') as HTMLElement | null
    prev?.remove()
    const el = document.createElement('div')
    el.className = 'thread-toast'
    el.textContent = message
    row.appendChild(el)
    setTimeout(() => el.remove(), 1200)
  }

  update(threads: ActiveThread[]) {
    // Hide truly-idle threads (per UX request: "only show active threads"),
    // but keep `stale` visible — stale means the adapter wedged or the
    // session disappeared, both of which the user should see (rendered
    // dimmed) rather than have silently disappear.
    this.threads = threads
      .filter(t => t.status !== 'idle')
      .sort((a, b) => STATUS_PRIORITY[b.status] - STATUS_PRIORITY[a.status])
    if (this.threads.length <= 1) this.expanded = false
    // Drop "revealed" state for threads that no longer exist so we don't
    // accumulate dead keys forever.
    const live = new Set(this.threads.map(t => this.rowKey(t)))
    for (const k of [...this.revealed]) {
      if (!live.has(k)) this.revealed.delete(k)
    }
    this.render()
  }

  private rowKey(t: ActiveThread): string {
    return `${t.tool}|${t.pid ?? ''}|${t.cwd ?? ''}|${t.title ?? ''}`
  }

  private render() {
    if (this.threads.length === 0) {
      this.cardEl.style.display = 'none'
      this.chevronEl.classList.remove('visible')
      this.onResize?.()
      return
    }

    const visible = this.expanded ? this.threads : [this.threads[0]]
    this.cardEl.innerHTML = visible.map(t => this.rowHtml(t)).join('')
    this.cardEl.style.display = 'block'

    if (this.threads.length > 1) {
      this.chevronEl.classList.add('visible')
      this.chevronGlyphEl.textContent = this.expanded
        ? '▲'
        : `+${this.threads.length - 1}`
      if (this.expanded) {
        this.chevronEl.classList.add('expanded')
      } else {
        this.chevronEl.classList.remove('expanded')
      }
    } else {
      this.chevronEl.classList.remove('visible', 'expanded')
    }

    this.onResize?.()
  }

  private rowHtml(t: ActiveThread): string {
    const title = this.displayTitle(t)
    const subtitle = `${STATUS_WORD[t.status]} · ${t.tool}`
    const staleCls = t.status === 'stale' ? ' stale' : ''
    const key = this.rowKey(t)
    const isRevealed = this.revealed.has(key)
    const isClickable = t.pid != null || (t.cwd && t.cwd.length > 0)
    const clickableCls = isClickable ? ' clickable' : ''
    const isDismissible = this.isDismissible(t)
    const statusHtml = isDismissible ? '' : this.statusIconHtml(t.status)
    const dismissHtml = isDismissible
      ? `<button class="thread-dismiss" type="button" title="Dismiss thread" aria-label="Dismiss thread" data-key="${this.esc(key)}">&times;</button>`
      : ''

    let detailsHtml = ''
    if (isRevealed) {
      const parts: string[] = []
      if (t.pid != null) parts.push(`pid ${t.pid}`)
      if (t.cwd) parts.push(this.prettyCwd(t.cwd))
      if (parts.length) {
        const fullCwd = t.cwd ?? ''
        detailsHtml = `<div class="thread-cwd" title="${this.esc(fullCwd)}">${this.esc(parts.join('  ·  '))}</div>`
      }
    }

    const pidAttr = t.pid != null ? ` data-pid="${t.pid}"` : ''
    return `
      <div class="thread-row${staleCls}${clickableCls}" data-key="${this.esc(key)}"${pidAttr}>
        <div class="thread-row-text">
          <div class="thread-title">${this.esc(title)}</div>
          <div class="thread-subtitle">${this.esc(subtitle)}</div>
          ${detailsHtml}
        </div>
        ${statusHtml}
        ${dismissHtml}
      </div>
    `
  }

  private isDismissible(t: ActiveThread): boolean {
    return t.status !== 'busy'
  }

  private displayTitle(t: ActiveThread): string {
    return (t.title?.trim() || 'Untitled thread').replace(/^#{1,6}\s+/, '')
  }

  /// Replace $HOME with ~ so cwd lines stay short. Also collapse extremely
  /// long paths from the middle.
  private prettyCwd(cwd: string): string {
    let s = cwd
    // Substitute home dir prefix. Renderer doesn't know HOME directly; the
    // common /Users/<user>/ or /home/<user>/ prefix is close enough.
    const macHome = s.match(/^(\/Users\/[^/]+)/)
    const linuxHome = s.match(/^(\/home\/[^/]+)/)
    const winHome = s.match(/^([A-Za-z]:\\Users\\[^\\]+)/)
    const prefix = macHome?.[1] || linuxHome?.[1] || winHome?.[1]
    if (prefix) s = '~' + s.slice(prefix.length)
    if (s.length > 48) {
      const head = s.slice(0, 18)
      const tail = s.slice(-26)
      s = `${head}…${tail}`
    }
    return s
  }

  private statusIconHtml(status: ThreadStatus): string {
    if (status === 'busy') {
      return '<div class="thread-status-icon spinner"></div>'
    }
    if (status === 'error' || status === 'waiting' || status === 'idle' || status === 'stale') {
      const cls = status === 'stale' ? 'idle' : status
      return `<div class="thread-status-icon dot ${cls}"></div>`
    }
    return ''
  }

  private esc(text: string): string {
    const div = document.createElement('div')
    div.textContent = text
    return div.innerHTML
  }
}
