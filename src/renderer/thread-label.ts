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
      white-space: nowrap;
      max-width: 250px;
      overflow: hidden;
      text-overflow: ellipsis;
      pointer-events: none;
      display: none;
      line-height: 1.4;
      z-index: 10;
    `
    container.appendChild(this.labelEl)
  }

  update(threads: ActiveThread[]) {
    if (threads.length === 0) {
      this.labelEl.style.display = 'none'
      return
    }

    const sorted = [...threads].sort(
      (a, b) => STATUS_PRIORITY[b.status] - STATUS_PRIORITY[a.status]
    )

    const lines = sorted.map(t => {
      const title = t.title ?? 'untitled'
      const opacity = t.status === 'stale' ? '0.5' : '1'
      return `<div style="opacity: ${opacity}">${this.escapeHtml(t.tool)} — ${this.escapeHtml(title)}</div>`
    })

    this.labelEl.innerHTML = lines.join('')
    this.labelEl.style.display = 'block'
  }

  private escapeHtml(text: string): string {
    const div = document.createElement('div')
    div.textContent = text
    return div.innerHTML
  }
}
