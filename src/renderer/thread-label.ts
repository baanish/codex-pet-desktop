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
  }

  update(threads: ActiveThread[]) {
    this.threads = threads
      .filter(t => t.status !== 'idle' && t.status !== 'stale')
      .sort((a, b) => STATUS_PRIORITY[b.status] - STATUS_PRIORITY[a.status])
    if (this.threads.length <= 1) this.expanded = false
    this.render()
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
    const title = t.title?.trim() || 'Untitled thread'
    const subtitle = `${STATUS_WORD[t.status]} · ${t.tool}`
    const staleCls = t.status === 'stale' ? ' stale' : ''
    return `
      <div class="thread-row${staleCls}">
        <div class="thread-row-text">
          <div class="thread-title">${this.esc(title)}</div>
          <div class="thread-subtitle">${this.esc(subtitle)}</div>
        </div>
        ${this.statusIconHtml(t.status)}
      </div>
    `
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
