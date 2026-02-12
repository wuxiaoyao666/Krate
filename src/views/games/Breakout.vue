<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
const canvasRef = ref<HTMLCanvasElement | null>(null)

// 游戏状态
const score = ref(0)
const lives = ref(1)
const isPlaying = ref(false)
const isPaused = ref(false)
const gameOver = ref(false)
const gameWin = ref(false)

// 配置常量
const PADDLE_WIDTH = 100
const PADDLE_HEIGHT = 10
const BALL_RADIUS = 6
const BRICK_WIDTH = 75
const BRICK_HEIGHT = 20
const BRICK_ROW_COUNT = 5
const BRICK_COLUMN_COUNT = 9
const BRICK_PADDING = 10
const BRICK_OFFSET_TOP = 60
const BRICK_OFFSET_LEFT = 35
const PADDLE_BOTTOM_GAP = 10
const KEYBOARD_PADDLE_SPEED = 9

// 运行时变量
let ctx: CanvasRenderingContext2D | null = null
let animationId = 0
let paddleX: number
let ball = { x: 0, y: 0, dx: 4, dy: -4, speed: 6 } // 增加了 speed 属性
let bricks: { x: number; y: number; status: number; color: string }[][] = []
let particles: Particle[] = []
let leftKeyPressed = false
let rightKeyPressed = false

const clampPaddleX = () => {
  if (!canvasRef.value) return
  if (paddleX < 0) paddleX = 0
  const maxX = canvasRef.value.width - PADDLE_WIDTH
  if (paddleX > maxX) paddleX = maxX
}

// 粒子类 (用于爆炸特效)
class Particle {
  x: number; y: number; dx: number; dy: number; life: number; color: string;
  constructor(x: number, y: number, color: string) {
    this.x = x; this.y = y; this.color = color;
    this.dx = (Math.random() - 0.5) * 4;
    this.dy = (Math.random() - 0.5) * 4;
    this.life = 1.0;
  }
  update() {
    this.x += this.dx; this.y += this.dy; this.life -= 0.02;
  }
  draw(ctx: CanvasRenderingContext2D) {
    ctx.globalAlpha = Math.max(0, this.life);
    ctx.fillStyle = this.color;
    ctx.beginPath(); ctx.arc(this.x, this.y, 2, 0, Math.PI * 2); ctx.fill(); ctx.closePath();
    ctx.globalAlpha = 1.0;
  }
}

// 初始化砖块
const initBricks = () => {
  const colors = ['#ef4444', '#f97316', '#eab308', '#22c55e', '#3b82f6'] // 每一行的颜色
  bricks = []
  for (let c = 0; c < BRICK_COLUMN_COUNT; c++) {
    bricks[c] = []
    for (let r = 0; r < BRICK_ROW_COUNT; r++) {
      bricks[c][r] = { x: 0, y: 0, status: 1, color: colors[r] }
    }
  }
}

// 游戏重置
const resetGame = () => {
  if (!canvasRef.value) return
  if (animationId) cancelAnimationFrame(animationId)
  score.value = 0
  lives.value = 1
  isPlaying.value = true
  isPaused.value = false
  gameOver.value = false
  gameWin.value = false
  leftKeyPressed = false
  rightKeyPressed = false
  paddleX = (canvasRef.value.width - PADDLE_WIDTH) / 2
  resetBall()
  initBricks()
  animate()
}

const resetBall = () => {
  if (!canvasRef.value) return
  ball.x = canvasRef.value.width / 2
  ball.y = canvasRef.value.height - 30
  ball.dx = 4 * (Math.random() > 0.5 ? 1 : -1)
  ball.dy = -4
  ball.speed = 6
}

// 核心绘制循环
const draw = () => {
  if (!ctx || !canvasRef.value) return

  // 清空画布 (带一点拖影效果会更酷，这里用全清空保持清晰)
  ctx.clearRect(0, 0, canvasRef.value.width, canvasRef.value.height)

  // 1. 绘制砖块
  for (let c = 0; c < BRICK_COLUMN_COUNT; c++) {
    for (let r = 0; r < BRICK_ROW_COUNT; r++) {
      if (bricks[c][r].status === 1) {
        const brickX = (c * (BRICK_WIDTH + BRICK_PADDING)) + BRICK_OFFSET_LEFT
        const brickY = (r * (BRICK_HEIGHT + BRICK_PADDING)) + BRICK_OFFSET_TOP
        bricks[c][r].x = brickX
        bricks[c][r].y = brickY

        ctx.beginPath()
        ctx.roundRect(brickX, brickY, BRICK_WIDTH, BRICK_HEIGHT, 4)
        ctx.fillStyle = bricks[c][r].color
        ctx.shadowBlur = 10
        ctx.shadowColor = bricks[c][r].color
        ctx.fill()
        ctx.shadowBlur = 0
        ctx.closePath()
      }
    }
  }

  // 2. 绘制球
  ctx.beginPath()
  ctx.arc(ball.x, ball.y, BALL_RADIUS, 0, Math.PI * 2)
  ctx.fillStyle = '#fff'
  ctx.shadowBlur = 15
  ctx.shadowColor = '#fff'
  ctx.fill()
  ctx.shadowBlur = 0
  ctx.closePath()

  // 3. 绘制挡板
  ctx.beginPath()
  ctx.roundRect(paddleX, canvasRef.value.height - PADDLE_HEIGHT - PADDLE_BOTTOM_GAP, PADDLE_WIDTH, PADDLE_HEIGHT, 5)
  ctx.fillStyle = '#10b981' // Emerald
  ctx.shadowBlur = 10
  ctx.shadowColor = '#10b981'
  ctx.fill()
  ctx.shadowBlur = 0
  ctx.closePath()

  // 4. 绘制粒子
  for (let i = particles.length - 1; i >= 0; i--) {
    particles[i].update()
    particles[i].draw(ctx)
    if (particles[i].life <= 0) particles.splice(i, 1)
  }
}

// 碰撞检测与逻辑更新
const update = () => {
  if (!canvasRef.value) return
  const { width, height } = canvasRef.value
  const paddleTop = height - PADDLE_HEIGHT - PADDLE_BOTTOM_GAP

  if (leftKeyPressed !== rightKeyPressed) {
    paddleX += leftKeyPressed ? -KEYBOARD_PADDLE_SPEED : KEYBOARD_PADDLE_SPEED
    clampPaddleX()
  }

  const nextX = ball.x + ball.dx
  const nextY = ball.y + ball.dy

  // 左右墙壁碰撞
  if (nextX > width - BALL_RADIUS || nextX < BALL_RADIUS) {
    ball.dx = -ball.dx
  }
  // 顶部碰撞
  if (nextY < BALL_RADIUS) {
    ball.dy = -ball.dy
  }

  // 挡板碰撞（只在球向下且穿过挡板上沿时生效，避免漏球被误判接中）
  const hitPaddle =
    ball.dy > 0 &&
    ball.y + BALL_RADIUS <= paddleTop &&
    nextY + BALL_RADIUS >= paddleTop &&
    nextX >= paddleX &&
    nextX <= paddleX + PADDLE_WIDTH

  if (hitPaddle) {
    let collidePoint = nextX - (paddleX + PADDLE_WIDTH / 2)
    collidePoint = collidePoint / (PADDLE_WIDTH / 2)

    const angle = collidePoint * (Math.PI / 3) // Max 60度
    ball.speed += 0.2 // 每次接球稍微加速
    ball.dx = ball.speed * Math.sin(angle)
    ball.dy = -ball.speed * Math.cos(angle)
  }

  // 漏球判定
  if (nextY + BALL_RADIUS >= height) {
    lives.value = 0
    gameOver.value = true
    isPlaying.value = false
    if (animationId) cancelAnimationFrame(animationId)
    return
  }

  // 移动球
  ball.x += ball.dx
  ball.y += ball.dy

  // 砖块碰撞
  let activeBricks = 0
  for (let c = 0; c < BRICK_COLUMN_COUNT; c++) {
    for (let r = 0; r < BRICK_ROW_COUNT; r++) {
      const b = bricks[c][r]
      if (b.status === 1) {
        activeBricks++
        const hitBrick =
          nextX + BALL_RADIUS > b.x &&
          nextX - BALL_RADIUS < b.x + BRICK_WIDTH &&
          nextY + BALL_RADIUS > b.y &&
          nextY - BALL_RADIUS < b.y + BRICK_HEIGHT

        if (hitBrick) {
          ball.dy = -ball.dy
          b.status = 0
          score.value += 10
          // 生成粒子
          for (let i = 0; i < 8; i++) {
            particles.push(new Particle(b.x + 37, b.y + 10, b.color))
          }
        }
      }
    }
  }

  if (activeBricks === 0) {
    gameWin.value = true
    isPlaying.value = false
    if (animationId) cancelAnimationFrame(animationId)
  }
}

const animate = () => {
  if (!isPlaying.value || isPaused.value) return
  draw()
  update()
  animationId = requestAnimationFrame(animate)
}

// 鼠标控制
const mouseMoveHandler = (e: MouseEvent) => {
  if (!canvasRef.value) return
  const rect = canvasRef.value.getBoundingClientRect()
  const relativeX = e.clientX - rect.left
  if (relativeX > 0 && relativeX < canvasRef.value.width) {
    paddleX = relativeX - PADDLE_WIDTH / 2
    clampPaddleX()
  }
}

const keydownHandler = (e: KeyboardEvent) => {
  if (e.key === 'Escape' && isPlaying.value) {
    isPaused.value = !isPaused.value
    if (!isPaused.value) {
      animate()
    }
    return
  }

  if (!isPlaying.value || isPaused.value) return

  if (e.code === 'ArrowLeft' || e.code === 'KeyA') {
    leftKeyPressed = true
    e.preventDefault()
  } else if (e.code === 'ArrowRight' || e.code === 'KeyD') {
    rightKeyPressed = true
    e.preventDefault()
  }
}

const keyupHandler = (e: KeyboardEvent) => {
  if (e.code === 'ArrowLeft' || e.code === 'KeyA') {
    leftKeyPressed = false
    e.preventDefault()
  } else if (e.code === 'ArrowRight' || e.code === 'KeyD') {
    rightKeyPressed = false
    e.preventDefault()
  }
}

const resumeGame = () => {
  if (!isPlaying.value || !isPaused.value) return
  isPaused.value = false
  animate()
}

onMounted(() => {
  if (canvasRef.value) {
    ctx = canvasRef.value.getContext('2d')
    paddleX = (canvasRef.value.width - PADDLE_WIDTH) / 2
    document.addEventListener('mousemove', mouseMoveHandler, false)
    document.addEventListener('keydown', keydownHandler, false)
    document.addEventListener('keyup', keyupHandler, false)
    initBricks()
    draw() // 绘制初始画面
  }
})

onUnmounted(() => {
  document.removeEventListener('mousemove', mouseMoveHandler)
  document.removeEventListener('keydown', keydownHandler)
  document.removeEventListener('keyup', keyupHandler)
  leftKeyPressed = false
  rightKeyPressed = false
  if (animationId) cancelAnimationFrame(animationId)
})
</script>

<template>
  <div class="fixed inset-0 bg-[#0F172A] flex flex-col items-center justify-center select-none overflow-hidden">
    <div class="absolute top-6 left-8 z-10 flex gap-8 font-mono text-xl font-bold tracking-widest pointer-events-none">
      <div class="text-emerald-400 drop-shadow-[0_0_5px_rgba(16,185,129,0.5)]">
        得分：{{ score }}
      </div>
      <div class="text-rose-400 drop-shadow-[0_0_5px_rgba(244,63,94,0.5)]">
        生命：{{ lives }}
      </div>
    </div>

    <canvas
      ref="canvasRef"
      width="800"
      height="600"
      class="bg-slate-900 rounded-xl shadow-2xl shadow-black border border-slate-700/50 cursor-none"
    ></canvas>

    <div
      v-if="!isPlaying || isPaused"
      class="absolute inset-0 bg-black/40 backdrop-blur-sm flex flex-col items-center justify-center z-20"
    >
      <h1
        class="text-6xl font-black italic tracking-tighter mb-4 text-transparent bg-clip-text bg-linear-to-r from-emerald-400 to-cyan-500 drop-shadow-lg"
      >
        {{ gameWin ? '胜利' : gameOver ? '游戏结束' : isPaused ? '已暂停' : '霓虹打砖块' }}
      </h1>

      <p class="text-slate-300 mb-10 font-mono text-lg">
        {{ gameWin ? `完美通关，最终得分：${score}` : gameOver ? `最终得分：${score}` : isPaused ? '按 ESC 或点击继续按钮恢复游戏' : '移动挡板并击碎全部砖块' }}
      </p>

      <button
        @click="isPaused ? resumeGame() : resetGame()"
        class="px-10 py-4 bg-emerald-500 hover:bg-emerald-400 text-slate-900 font-bold text-xl rounded-full
               transition-all hover:scale-105 active:scale-95 shadow-[0_0_20px_rgba(16,185,129,0.4)]"
      >
        {{ gameOver || gameWin ? '再玩一次' : isPaused ? '继续游戏' : '开始游戏' }}
      </button>
    </div>

    <div class="absolute bottom-4 text-slate-600 text-xs font-mono">
      鼠标、方向键或 A/D 控制 · ESC 暂停或继续
    </div>
  </div>
</template>
