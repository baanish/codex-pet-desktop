import { PetSpriteAtlas, SpriteAnimation } from '../shared/types'

export class SpriteEngine {
  private canvas: HTMLCanvasElement
  private ctx: CanvasRenderingContext2D
  private image: HTMLImageElement
  private atlas: PetSpriteAtlas
  private currentAnimation: SpriteAnimation | null = null
  private currentFrame = 0
  private elapsedMs = 0
  private lastTimestamp = 0
  private running = false

  constructor(canvas: HTMLCanvasElement, atlas: PetSpriteAtlas) {
    this.canvas = canvas
    this.ctx = canvas.getContext('2d')!
    this.atlas = atlas
    this.image = new Image()

    canvas.width = atlas.cellWidth
    canvas.height = atlas.cellHeight
  }

  loadSpritesheet(src: string): Promise<void> {
    return new Promise((resolve, reject) => {
      this.image.onload = () => resolve()
      this.image.onerror = reject
      this.image.src = src
    })
  }

  setAnimation(name: string) {
    const anim = this.atlas.animations[name]
    if (!anim) return
    if (this.currentAnimation === anim) return

    this.currentAnimation = anim
    this.currentFrame = 0
    this.elapsedMs = 0
  }

  start() {
    if (this.running) return
    this.running = true
    this.lastTimestamp = performance.now()
    this.tick()
  }

  stop() {
    this.running = false
  }

  private tick = () => {
    if (!this.running) return

    const now = performance.now()
    const delta = now - this.lastTimestamp
    this.lastTimestamp = now

    this.elapsedMs += delta

    if (this.currentAnimation) {
      const frameDuration = this.currentAnimation.frameDurations[this.currentFrame]
      if (this.elapsedMs >= frameDuration) {
        this.elapsedMs -= frameDuration
        this.currentFrame = (this.currentFrame + 1) % this.currentAnimation.frames
      }

      this.draw()
    }

    requestAnimationFrame(this.tick)
  }

  private draw() {
    if (!this.currentAnimation) return

    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height)

    const { cellWidth, cellHeight } = this.atlas
    const col = this.currentFrame
    const row = this.currentAnimation.row

    this.ctx.drawImage(
      this.image,
      col * cellWidth,
      row * cellHeight,
      cellWidth,
      cellHeight,
      0,
      0,
      cellWidth,
      cellHeight
    )
  }
}
