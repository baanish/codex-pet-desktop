export class DragHandler {
  private el: HTMLElement
  private isDragging = false
  private screenStartX = 0
  private screenStartY = 0
  private winStartX = 0
  private winStartY = 0
  private onInteraction: () => void

  constructor(
    el: HTMLElement,
    onInteraction: () => void
  ) {
    this.el = el
    this.onInteraction = onInteraction
    this.el.style.cursor = 'grab'

    this.el.addEventListener('pointerdown', this.onPointerDown)
    window.addEventListener('pointermove', this.onPointerMove)
    window.addEventListener('pointerup', this.onPointerUp)
    window.addEventListener('pointercancel', this.onPointerUp)
  }

  private onPointerDown = (e: PointerEvent) => {
    if (e.button !== 0) return
    e.preventDefault()

    this.isDragging = true
    this.el.style.cursor = 'grabbing'

    // Record screen-space mouse position and current window position
    this.screenStartX = e.screenX
    this.screenStartY = e.screenY
    this.winStartX = window.screenX
    this.winStartY = window.screenY

    this.onInteraction()
  }

  private onPointerMove = (e: PointerEvent) => {
    if (!this.isDragging) return

    const dx = e.screenX - this.screenStartX
    const dy = e.screenY - this.screenStartY

    const newX = this.winStartX + dx
    const newY = this.winStartY + dy

    window.petBridge.setWindowPosition(newX, newY)
  }

  private onPointerUp = () => {
    if (!this.isDragging) return

    this.isDragging = false
    this.el.style.cursor = 'grab'

    // Save final position
    window.petBridge.savePosition({ x: window.screenX, y: window.screenY })

    // Re-poll so animation reverts to idle if nothing is busy
    this.onInteraction()
  }

  destroy() {
    this.el.removeEventListener('pointerdown', this.onPointerDown)
    window.removeEventListener('pointermove', this.onPointerMove)
    window.removeEventListener('pointerup', this.onPointerUp)
    window.removeEventListener('pointercancel', this.onPointerUp)
  }
}
