export class DragHandler {
  private el: HTMLElement
  private isDragging = false
  private offsetX = 0
  private offsetY = 0
  private onDragEnd: (position: { x: number; y: number }) => void
  private onInteraction: () => void

  constructor(
    el: HTMLElement,
    onDragEnd: (position: { x: number; y: number }) => void,
    onInteraction: () => void
  ) {
    this.el = el
    this.onDragEnd = onDragEnd
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

    const rect = this.el.getBoundingClientRect()
    this.offsetX = e.clientX - rect.left
    this.offsetY = e.clientY - rect.top

    this.onInteraction()
  }

  private onPointerMove = (e: PointerEvent) => {
    if (!this.isDragging) return

    const x = e.clientX - this.offsetX
    const y = e.clientY - this.offsetY

    const maxX = window.innerWidth - this.el.offsetWidth
    const maxY = window.innerHeight - this.el.offsetHeight
    const clampedX = Math.max(0, Math.min(x, maxX))
    const clampedY = Math.max(0, Math.min(y, maxY))

    this.el.style.transform = `translate(${clampedX}px, ${clampedY}px)`
  }

  private onPointerUp = () => {
    if (!this.isDragging) return

    this.isDragging = false
    this.el.style.cursor = 'grab'

    const rect = this.el.getBoundingClientRect()
    this.onDragEnd({ x: rect.left, y: rect.top })
  }

  destroy() {
    this.el.removeEventListener('pointerdown', this.onPointerDown)
    window.removeEventListener('pointermove', this.onPointerMove)
    window.removeEventListener('pointerup', this.onPointerUp)
    window.removeEventListener('pointercancel', this.onPointerUp)
  }
}
