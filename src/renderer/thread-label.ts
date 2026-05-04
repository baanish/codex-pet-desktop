import { ActiveThread, ThreadStatus } from '../shared/types'

const STATUS_PRIORITY: Record<ThreadStatus, number> = {
  error: 4,
  busy: 3,
  waiting: 2,
  stale: 1,
  idle: 0
}

export class ThreadLabel {
  private container: HTMLElement
  private labelEl: HTMLElement
  private expanded = false
  private threads: ActiveThread[] = []

  constructor(container: HTMLElement) {
    this.container = container
    this.labelEl = document.createElement('div')
    this.labelEl.style.cssText = `
      position: absolute;
      top: 0;
      left: 50%;
      transform: translateX(-50%);
      background: rgba(0, 0, 0, 0.8);
      color: #fff;
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
      font-size: 11px;
      padding: 4px 8px;
      border-radius: 6px;
      pointer-events: auto;
      cursor: pointer;
      display: none;
      line-height: 1.4;
      z-index: 10;
      user-select: none;
    `
    this.labelEl.addEventListener('click', (e) => {
      e.stopPropagation()
      this.expanded = !this.expanded
      this.render()
    })
    container.appendChild(this.labelEl)
  }

  update(threads: ActiveThread[]) {
    this.threads = [...threads].sort(
      (a, b) => STATUS_PRIORITY[b.status] - STATUS_PRIORITY[a.status]
    )
    this.render()
  }

  private render() {
    if (this.threads.length === 0) {
      this.labelEl.style.display = 'none'
      return
    }

    const lines = this.expanded ? this.threads : [this.threads[0]]

    const html = lines.map(t => {
      const title = t.title ?? 'untitled'
      const opacity = t.status === 'stale' ? '0.5' : '1'
      return `<div style="opacity: ${opacity}; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 250px;">${this.escapeHtml(t.tool)} — ${this.escapeHtml(title)}</div>`
    }).join('')

    const indicator = this.threads.length > 1
      ? `<div style="text-align: center; font-size: 9px; opacity: 0.6; margin-top: 2px;">${this.expanded ? '▲' : '▼'}</div>`
      : ''

    this.labelEl.innerHTML = html + indicator
    this.labelEl.style.display = 'block'
  }

  private escapeHtml(text: string): string {
    const div = document.createElement('div')
    div.textContent = text
    return div.innerHTML
  }
}
