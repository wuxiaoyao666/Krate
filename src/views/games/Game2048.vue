<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { NButton, NIcon } from 'naive-ui'
import { Restart, Undo } from '@vicons/carbon'

type Dir = 'left' | 'right' | 'up' | 'down'

interface Tile {
  id: number
  r: number
  c: number
  val: number
  isNew?: boolean
  isMerged?: boolean
}

interface Snapshot {
  tiles: Tile[]
  score: number
  best: number
  gameOver: boolean
  won: boolean
  keepPlaying: boolean
}

const GRID_SIZE = 4
const CELL = 84
const GAP = 12
const ANIM_MS = 140
const BEST_KEY = 'krate_2048_best'
const MAX_HISTORY = 30

const boardPx = computed(() => GRID_SIZE * (CELL + GAP) + GAP)

const tiles = ref<Tile[]>([])
const score = ref(0)
const bestScore = ref<number>(Number(localStorage.getItem(BEST_KEY) || '0'))
const gameOver = ref(false)
const won = ref(false)
const keepPlaying = ref(false)

const history = ref<Snapshot[]>([])
const isMoving = ref(false)

const boardRef = ref<HTMLElement | null>(null)

let nextId = 1

function key(r: number, c: number) {
  return `${r},${c}`
}

function cloneTiles(list: Tile[]) {
  return list.map((t) => ({ ...t }))
}

function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

function emptyCells(): Array<{ r: number; c: number }> {
  const occ = new Set(tiles.value.map((t) => key(t.r, t.c)))
  const res: Array<{ r: number; c: number }> = []
  for (let r = 0; r < GRID_SIZE; r++) {
    for (let c = 0; c < GRID_SIZE; c++) {
      if (!occ.has(key(r, c))) res.push({ r, c })
    }
  }
  return res
}

function spawnRandomTile() {
  const empties = emptyCells()
  if (empties.length === 0) return
  const pos = empties[Math.floor(Math.random() * empties.length)]
  const val = Math.random() < 0.9 ? 2 : 4
  tiles.value.push({
    id: nextId++,
    r: pos.r,
    c: pos.c,
    val,
    isNew: true,
    isMerged: false
  })
}

function buildMatrix(): number[][] {
  const m = Array.from({ length: GRID_SIZE }, () => Array(GRID_SIZE).fill(0))
  for (const t of tiles.value) m[t.r][t.c] = t.val
  return m
}

function hasAvailableMoves(): boolean {
  const m = buildMatrix()

  // 有空位
  for (let r = 0; r < GRID_SIZE; r++) {
    for (let c = 0; c < GRID_SIZE; c++) {
      if (m[r][c] === 0) return true
    }
  }

  // 相邻可合并
  for (let r = 0; r < GRID_SIZE; r++) {
    for (let c = 0; c < GRID_SIZE; c++) {
      const v = m[r][c]
      if (r + 1 < GRID_SIZE && m[r + 1][c] === v) return true
      if (c + 1 < GRID_SIZE && m[r][c + 1] === v) return true
    }
  }
  return false
}

function updateWinLose() {
  const maxVal = tiles.value.reduce((mx, t) => Math.max(mx, t.val), 0)
  if (!won.value && maxVal >= 2048) {
    won.value = true
    keepPlaying.value = false
  }
  gameOver.value = !hasAvailableMoves()
}

function initGame() {
  tiles.value = []
  score.value = 0
  gameOver.value = false
  won.value = false
  keepPlaying.value = false
  history.value = []
  nextId = 1
  spawnRandomTile()
  spawnRandomTile()
}

function undo() {
  if (isMoving.value) return
  const snap = history.value.pop()
  if (!snap) return

  tiles.value = cloneTiles(snap.tiles)
  score.value = snap.score
  bestScore.value = snap.best
  gameOver.value = snap.gameOver
  won.value = snap.won
  keepPlaying.value = snap.keepPlaying
}

function getLinePositions(dir: Dir, idx: number): Array<{ r: number; c: number }> {
  const res: Array<{ r: number; c: number }> = []
  if (dir === 'left' || dir === 'right') {
    const r = idx
    const cols = dir === 'left' ? [0, 1, 2, 3] : [3, 2, 1, 0]
    for (const c of cols) res.push({ r, c })
  } else {
    const c = idx
    const rows = dir === 'up' ? [0, 1, 2, 3] : [3, 2, 1, 0]
    for (const r of rows) res.push({ r, c })
  }
  return res
}

async function move(dir: Dir) {
  if (isMoving.value) return
  if (gameOver.value) return
  if (won.value && !keepPlaying.value) return

  isMoving.value = true

  // 清标记（让 new/merge 动画能重复触发）
  tiles.value.forEach((t) => {
    t.isNew = false
    t.isMerged = false
  })

  const posMap = new Map<string, Tile>()
  for (const t of tiles.value) posMap.set(key(t.r, t.c), t)

  let anyMoved = false
  let gained = 0
  const toRemove = new Set<number>()

  // 记录历史（只有真正发生移动/合并才会保留）
  const before = {
    tiles: cloneTiles(tiles.value),
    score: score.value,
    best: bestScore.value,
    gameOver: gameOver.value,
    won: won.value,
    keepPlaying: keepPlaying.value
  } satisfies Snapshot

  for (let line = 0; line < GRID_SIZE; line++) {
    const positions = getLinePositions(dir, line)

    let targetIdx = 0
    let lastPlaced: Tile | null = null
    let lastPos: { r: number; c: number } | null = null

    for (const p of positions) {
      const t = posMap.get(key(p.r, p.c))
      if (!t) continue

      // 合并：和上一块一样且上一块本轮还没合并过
      if (lastPlaced && !lastPlaced.isMerged && lastPlaced.val === t.val) {
        // 当前块移动到 lastPos 并被吸收
        if (lastPos && (t.r !== lastPos.r || t.c !== lastPos.c)) anyMoved = true
        t.r = lastPos!.r
        t.c = lastPos!.c

        lastPlaced.val *= 2
        lastPlaced.isMerged = true
        gained += lastPlaced.val

        toRemove.add(t.id)
        // targetIdx 不变（下一个放到紧接着的位置）
        continue
      }

      // 普通移动：放到 targetIdx 对应位置
      const target = positions[targetIdx]
      if (t.r !== target.r || t.c !== target.c) anyMoved = true
      t.r = target.r
      t.c = target.c

      lastPlaced = t
      lastPos = target
      targetIdx++
    }
  }

  // 没有发生任何变化就直接结束
  if (!anyMoved && gained === 0) {
    isMoving.value = false
    return
  }

  // 真正发生变化才入栈
  history.value.push(before)
  if (history.value.length > MAX_HISTORY) history.value.shift()

  score.value += gained
  if (score.value > bestScore.value) {
    bestScore.value = score.value
    localStorage.setItem(BEST_KEY, String(bestScore.value))
  }

  // 等动画结束后移除被合并掉的 tile，再生成新 tile
  await sleep(ANIM_MS)
  tiles.value = tiles.value.filter((t) => !toRemove.has(t.id))
  spawnRandomTile()
  updateWinLose()

  isMoving.value = false
}

function continueGame() {
  keepPlaying.value = true
}

function onKeydown(e: KeyboardEvent) {
  const k = e.key.toLowerCase()

  // 撤销：Ctrl/Cmd + Z
  if ((e.ctrlKey || e.metaKey) && k === 'z') {
    e.preventDefault()
    undo()
    return
  }

  const map: Record<string, Dir | undefined> = {
    arrowleft: 'left',
    a: 'left',
    arrowright: 'right',
    d: 'right',
    arrowup: 'up',
    w: 'up',
    arrowdown: 'down',
    s: 'down'
  }
  const dir = map[k]
  if (!dir) return

  e.preventDefault()
  void move(dir)
}

// 触控 / 鼠标拖动（swipe）
let startX = 0
let startY = 0
let tracking = false

function onPointerDown(e: PointerEvent) {
  tracking = true
  startX = e.clientX
  startY = e.clientY
  boardRef.value?.setPointerCapture?.(e.pointerId)
}

function onPointerUp(e: PointerEvent) {
  if (!tracking) return
  tracking = false

  const dx = e.clientX - startX
  const dy = e.clientY - startY
  const ax = Math.abs(dx)
  const ay = Math.abs(dy)

  const TH = 28
  if (Math.max(ax, ay) < TH) return

  const dir: Dir = ax > ay ? (dx > 0 ? 'right' : 'left') : dy > 0 ? 'down' : 'up'
  void move(dir)
}

function bgCellStyle(idx: number) {
  const r = Math.floor(idx / GRID_SIZE)
  const c = idx % GRID_SIZE
  return {
    width: `${CELL}px`,
    height: `${CELL}px`,
    left: `${GAP + c * (CELL + GAP)}px`,
    top: `${GAP + r * (CELL + GAP)}px`
  }
}

function tileStyle(t: Tile) {
  const x = GAP + t.c * (CELL + GAP)
  const y = GAP + t.r * (CELL + GAP)
  return {
    width: `${CELL}px`,
    height: `${CELL}px`,
    transform: `translate(${x}px, ${y}px)`,
    transitionDuration: `${ANIM_MS}ms`,
    zIndex: t.isMerged || t.isNew ? 20 : 10
  }
}

function tileColorClass(val: number) {
  switch (val) {
    case 2:
      return 'bg-[#eee4da] text-[#776e65]'
    case 4:
      return 'bg-[#ede0c8] text-[#776e65]'
    case 8:
      return 'bg-[#f2b179] text-[#f9f6f2]'
    case 16:
      return 'bg-[#f59563] text-[#f9f6f2]'
    case 32:
      return 'bg-[#f67c5f] text-[#f9f6f2]'
    case 64:
      return 'bg-[#f65e3b] text-[#f9f6f2]'
    case 128:
      return 'bg-[#edcf72] text-[#f9f6f2]'
    case 256:
      return 'bg-[#edcc61] text-[#f9f6f2]'
    case 512:
      return 'bg-[#edc850] text-[#f9f6f2]'
    case 1024:
      return 'bg-[#edc53f] text-[#f9f6f2]'
    case 2048:
      return 'bg-[#edc22e] text-[#f9f6f2]'
    default:
      return 'bg-[#3c3a32] text-[#f9f6f2]'
  }
}

function tileTextClass(val: number) {
  if (val < 100) return 'text-4xl'
  if (val < 1000) return 'text-3xl'
  if (val < 10000) return 'text-2xl'
  return 'text-xl'
}

onMounted(() => {
  initGame()
  window.addEventListener('keydown', onKeydown, { passive: false })
  boardRef.value?.addEventListener('pointerdown', onPointerDown)
  boardRef.value?.addEventListener('pointerup', onPointerUp)
  boardRef.value?.addEventListener('pointercancel', onPointerUp)
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown as any)
  boardRef.value?.removeEventListener('pointerdown', onPointerDown)
  boardRef.value?.removeEventListener('pointerup', onPointerUp)
  boardRef.value?.removeEventListener('pointercancel', onPointerUp)
})
</script>

<template>
  <div class="min-h-screen w-full bg-[#faf8ef] flex items-center justify-center p-5 select-none">
    <div class="w-full max-w-[520px]">
      <!-- Header -->
      <div class="flex items-end justify-between gap-4">
        <div>
          <div class="text-5xl font-extrabold text-[#776e65] leading-none">2048</div>
          <div class="text-sm text-slate-500 mt-2">合并数字，冲击 2048（支持 WASD / 方向键 / 滑动）</div>
        </div>

        <div class="flex gap-2">
          <div class="bg-[#bbada0] rounded-lg px-3 py-2 text-center min-w-[96px]">
            <div class="text-[11px] font-bold tracking-widest text-[#eee4da]">得分</div>
            <div class="text-xl font-extrabold text-white">{{ score }}</div>
          </div>
          <div class="bg-[#bbada0] rounded-lg px-3 py-2 text-center min-w-[96px]">
            <div class="text-[11px] font-bold tracking-widest text-[#eee4da]">最高分</div>
            <div class="text-xl font-extrabold text-white">{{ bestScore }}</div>
          </div>
        </div>
      </div>

      <!-- Controls -->
      <div class="mt-4 flex items-center justify-between gap-3">
        <div class="text-xs text-slate-500 flex items-center gap-2">
          <span class="px-2 py-1 rounded bg-white/70 border border-slate-200">WASD</span>
          <span>或</span>
          <span class="px-2 py-1 rounded bg-white/70 border border-slate-200">方向键</span>
          <span class="hidden sm:inline">（Ctrl/Cmd + Z 撤销）</span>
        </div>

        <div class="flex gap-2">
          <NButton size="small" secondary :disabled="history.length === 0 || isMoving" @click="undo">
            <template #icon>
              <NIcon :component="Undo" />
            </template>
            撤销
          </NButton>

          <NButton size="small" type="primary" color="#8f7a66" :disabled="isMoving" @click="initGame">
            <template #icon>
              <NIcon :component="Restart" />
            </template>
            新游戏
          </NButton>
        </div>
      </div>

      <!-- Board -->
      <div
        ref="boardRef"
        class="relative mt-4 bg-[#bbada0] rounded-xl shadow-xl"
        :style="{ width: boardPx + 'px', height: boardPx + 'px', touchAction: 'none' }"
      >
        <!-- background cells -->
        <div
          v-for="i in GRID_SIZE * GRID_SIZE"
          :key="'bg-' + i"
          class="absolute rounded-lg bg-[#cdc1b4]/70"
          :style="bgCellStyle(i - 1)"
        />

        <!-- tiles -->
        <div
          v-for="t in tiles"
          :key="t.id"
          class="absolute top-0 left-0 rounded-lg flex items-center justify-center font-extrabold shadow-md will-change-transform transition-transform ease-in-out"
          :class="[tileColorClass(t.val), tileTextClass(t.val), { 'tile-new': t.isNew, 'tile-merged': t.isMerged }]"
          :style="tileStyle(t)"
        >
          {{ t.val }}
        </div>

        <!-- WIN overlay -->
        <div
          v-if="won && !keepPlaying"
          class="absolute inset-0 rounded-xl bg-[#faf8ef]/85 backdrop-blur-sm flex items-center justify-center"
        >
          <div class="text-center px-6">
            <div class="text-4xl font-extrabold text-[#776e65]">你赢了！</div>
            <div class="mt-2 text-[#776e65]">达到 2048</div>
            <div class="mt-5 flex items-center justify-center gap-3">
              <NButton type="primary" color="#8f7a66" @click="continueGame">继续挑战</NButton>
              <NButton secondary @click="initGame">新游戏</NButton>
            </div>
          </div>
        </div>

        <!-- GAME OVER overlay -->
        <div
          v-if="gameOver"
          class="absolute inset-0 rounded-xl bg-[#faf8ef]/85 backdrop-blur-sm flex items-center justify-center"
        >
          <div class="text-center px-6">
            <div class="text-4xl font-extrabold text-[#776e65]">结束啦</div>
            <div class="mt-2 text-[#776e65]">没有可移动的步骤了</div>
            <div class="mt-5 flex items-center justify-center gap-3">
              <NButton type="primary" color="#8f7a66" @click="initGame">再来一局</NButton>
              <NButton secondary :disabled="history.length === 0" @click="undo">撤销一步</NButton>
            </div>
          </div>
        </div>
      </div>

      <div class="mt-3 text-xs text-slate-500">
        小提示：合并得到更大的数字；一行连续相同数字时，每个格子每回合只合并一次。
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 新出现 tile 的 pop 动画 */
.tile-new {
  animation: pop 140ms ease-out;
}

/* 合并后的 pop 动画 */
.tile-merged {
  animation: pop 160ms ease-out;
}

@keyframes pop {
  0% {
    transform: scale(0.92);
  }
  60% {
    transform: scale(1.06);
  }
  100% {
    transform: scale(1);
  }
}
</style>
