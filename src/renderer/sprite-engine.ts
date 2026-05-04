import { PetSpriteAtlas, SpriteAnimation } from '../shared/types'

export class SpriteEngine {
  private canvas: HTMLCanvasElement
  private ctx: CanvasRenderingContext2D
  private image: HTMLImageElement
  private atlas: PetSpriteAtlas
  private currentAnimation: SpriteAnimation | null = null
  private currentAnimationName = ''
  private currentFrame = 0
  private direction: 1 | -1 = 1
  private running = false
  private scale = 1
  private speeds: Record<string, number> = {}
  private timer: ReturnType<typeof setTimeout> | null = null

  constructor(canvas: HTMLCanvasElement, atlas: PetSpriteAtlas) {
    this.canvas = canvas
    this.ctx = canvas.getContext('2d')!
    this.atlas = atlas
    this.image = new Image()

    canvas.width = atlas.cellWidth
    canvas.height = atlas.cellHeight
    this.disableSmoothing()
  }

  private disableSmoothing() {
    this.ctx.imageSmoothingEnabled = false
    ;(this.ctx as any).webkitImageSmoothingEnabled = false
    ;(this.ctx as any).mozImageSmoothingEnabled = false
  }

  setScale(scale: number) {
    this.scale = scale
    // Keep the drawing buffer at full source resolution; scale via CSS so
    // image-rendering: pixelated handles the downsample without bilinear blur.
    this.canvas.width = this.atlas.cellWidth
    this.canvas.height = this.atlas.cellHeight
    this.canvas.style.width = `${Math.round(this.atlas.cellWidth * scale)}px`
    this.canvas.style.height = `${Math.round(this.atlas.cellHeight * scale)}px`
    this.disableSmoothing()
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
    this.currentAnimationName = name
    this.currentFrame = 0
    this.direction = 1

    if (this.running) {
      this.draw()
      this.scheduleNext()
    }
  }

  setAnimationSpeed(name: string, speed: number) {
    this.speeds[name] = speed
  }

  private effectiveDuration(): number {
    const anim = this.currentAnimation!
    const base = anim.frameDurations[this.currentFrame]
    const speed = this.speeds[this.currentAnimationName] ?? 1
    return speed > 0 ? base / speed : base
  }

  start() {
    if (this.running) return
    this.running = true
    if (this.currentAnimation) this.draw()
    this.scheduleNext()
  }

  stop() {
    this.running = false
    if (this.timer) {
      clearTimeout(this.timer)
      this.timer = null
    }
  }

  // Wake only at frame-change boundaries instead of running a 60Hz rAF loop.
  // Pet frames last 180–480ms, so this drops idle CPU/GPU dramatically.
  private scheduleNext() {
    if (this.timer) clearTimeout(this.timer)
    if (!this.running || !this.currentAnimation) return
    const wait = Math.max(16, this.effectiveDuration())
    this.timer = setTimeout(() => {
      if (!this.running || !this.currentAnimation) return
      this.advanceFrame()
      this.draw()
      this.scheduleNext()
    }, wait)
  }

  private advanceFrame() {
    const anim = this.currentAnimation!
    if (!anim.pingpong) {
      this.currentFrame = (this.currentFrame + 1) % anim.frames
      return
    }
    // Ping-pong: 0 → frames-1 → 0 → ... bouncing without holding endpoints.
    let next = this.currentFrame + this.direction
    if (next >= anim.frames) {
      next = anim.frames - 2
      this.direction = -1
    } else if (next < 0) {
      next = 1
      this.direction = 1
    }
    this.currentFrame = Math.max(0, Math.min(anim.frames - 1, next))
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
      this.canvas.width,
      this.canvas.height
    )
  }
}
