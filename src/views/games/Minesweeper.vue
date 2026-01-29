<script setup lang="ts">
import { ref, computed, onUnmounted, reactive, watch } from 'vue'
import {
  NButton, NIcon, NSelect, NModal, NInputNumber, NCard, useMessage
} from 'naive-ui'
import {
  Settings, FaceSatisfied, FaceDizzy, FaceCool, CheckmarkFilled, Close
} from '@vicons/carbon'

// === 类型定义 ===
type DifficultyKey = 'easy' | 'medium' | 'hard' | 'custom'

interface Cell {
  row: number
  col: number
  isMine: boolean
  isRevealed: boolean
  isFlagged: boolean
  isExploded: boolean
  neighborMines: number
  isFlashing: boolean // 闪烁控制
}

const message = useMessage()

// === 游戏配置 ===
const difficulties: Record<DifficultyKey, { label: string, rows: number, cols: number, mines: number }> = {
  easy: { label: '简单 (9x9)', rows: 9, cols: 9, mines: 10 },
  medium: { label: '中等 (16x16)', rows: 16, cols: 16, mines: 40 },
  hard: { label: '困难 (30x16)', rows: 16, cols: 30, mines: 99 },
  custom: { label: '自定义', rows: 20, cols: 30, mines: 100 }
}

const diffOptions = [
  { label: '简单 (9x9)', value: 'easy' },
  { label: '中等 (16x16)', value: 'medium' },
  { label: '困难 (30x16)', value: 'hard' },
  { label: '自定义配置', value: 'custom' },
]

// === 状态 ===
const currentDiff = ref<DifficultyKey>('easy')
const showCustomModal = ref(false)
const customConfig = reactive({ rows: 16, cols: 30, mines: 99 }) // 默认自定义为困难

const board = ref<Cell[][]>([])
const gameState = ref<'idle' | 'playing' | 'won' | 'lost'>('idle')
const timer = ref(0)
const flagsUsed = ref(0)
let timerInterval: any = null

const minesLeft = computed(() => {
  const total = currentDiff.value === 'custom' ? customConfig.mines : difficulties[currentDiff.value].mines
  return total - flagsUsed.value
})

const density = computed(() => {
  const totalCells = customConfig.rows * customConfig.cols
  if (totalCells === 0) return '0%'
  return ((customConfig.mines / totalCells) * 100).toFixed(1) + '%'
})

// === 核心逻辑 ===

const initGame = () => {
  stopTimer()
  timer.value = 0
  flagsUsed.value = 0
  gameState.value = 'idle'

  const cfg = currentDiff.value === 'custom' ? customConfig : difficulties[currentDiff.value]

  // 生成空板
  const newBoard: Cell[][] = []
  for (let r = 0; r < cfg.rows; r++) {
    const row: Cell[] = []
    for (let c = 0; c < cfg.cols; c++) {
      row.push({
        row: r, col: c,
        isMine: false, isRevealed: false, isFlagged: false, isExploded: false,
        neighborMines: 0,
        isFlashing: false
      })
    }
    newBoard.push(row)
  }
  board.value = newBoard
}

// 保证第一步不踩雷
const placeMines = (excludeR: number, excludeC: number) => {
  const cfg = currentDiff.value === 'custom' ? customConfig : difficulties[currentDiff.value]
  let minesPlaced = 0

  // 保护区：点击位置及其周围一圈
  const isSafeZone = (r: number, c: number) => {
    return Math.abs(r - excludeR) <= 1 && Math.abs(c - excludeC) <= 1
  }

  while (minesPlaced < cfg.mines) {
    const r = Math.floor(Math.random() * cfg.rows)
    const c = Math.floor(Math.random() * cfg.cols)
    if (board.value[r][c].isMine || isSafeZone(r, c)) continue
    board.value[r][c].isMine = true
    minesPlaced++
  }

  // 计算数字
  const rows = board.value.length
  const cols = board.value[0].length
  for (let r = 0; r < rows; r++) {
    for (let c = 0; c < cols; c++) {
      if (board.value[r][c].isMine) continue
      let count = 0
      getNeighbors(r, c).forEach(n => { if (n.isMine) count++ })
      board.value[r][c].neighborMines = count
    }
  }
}

const getNeighbors = (r: number, c: number): Cell[] => {
  const neighbors: Cell[] = []
  const rows = board.value.length
  const cols = board.value[0].length
  for (let dr = -1; dr <= 1; dr++) {
    for (let dc = -1; dc <= 1; dc++) {
      if (dr === 0 && dc === 0) continue
      const nr = r + dr
      const nc = c + dc
      if (nr >= 0 && nr < rows && nc >= 0 && nc < cols) {
        neighbors.push(board.value[nr][nc])
      }
    }
  }
  return neighbors
}

// 左键：翻开
const handleLeftClick = (r: number, c: number) => {
  if (gameState.value === 'won' || gameState.value === 'lost') return
  const cell = board.value[r][c]
  if (cell.isFlagged || cell.isRevealed) return

  if (gameState.value === 'idle') {
    gameState.value = 'playing'
    startTimer()
    placeMines(r, c)
  }
  reveal(r, c)
}

const reveal = (r: number, c: number) => {
  const cell = board.value[r][c]
  if (cell.isRevealed || cell.isFlagged) return

  cell.isRevealed = true

  if (cell.isMine) {
    gameOver(false, r, c)
    return
  }

  if (cell.neighborMines === 0) {
    getNeighbors(r, c).forEach(n => reveal(n.row, n.col))
  }

  checkWin()
}

// 右键逻辑：插旗 + 和弦
const handleRightClick = (e: Event, r: number, c: number) => {
  e.preventDefault()
  if (gameState.value === 'won' || gameState.value === 'lost') return

  const cell = board.value[r][c]

  // 1. 未翻开 -> 切换插旗
  if (!cell.isRevealed) {
    cell.isFlagged = !cell.isFlagged
    flagsUsed.value += cell.isFlagged ? 1 : -1
    return
  }

  // 2. 已翻开且有数字 -> 尝试和弦
  if (cell.isRevealed && cell.neighborMines > 0) {
    triggerChord(cell)
  }
}

// 和弦逻辑：检查周围旗子数量
const triggerChord = (cell: Cell) => {
  const neighbors = getNeighbors(cell.row, cell.col)
  const flagCount = neighbors.filter(n => n.isFlagged).length

  // 如果旗子数量 == 数字 -> 翻开周围未插旗的格子
  if (flagCount === cell.neighborMines) {
    neighbors.forEach(n => {
      if (!n.isFlagged && !n.isRevealed) {
        reveal(n.row, n.col)
      }
    })
  } else {
    // 旗子数量不对 -> 闪烁提醒
    neighbors.forEach(n => {
      if (!n.isRevealed && !n.isFlagged) {
        n.isFlashing = true
        setTimeout(() => { n.isFlashing = false }, 150)
      }
    })
  }
}

const gameOver = (win: boolean, hitR = -1, hitC = -1) => {
  stopTimer()
  gameState.value = win ? 'won' : 'lost'

  if (win) {
    message.success('已通关，太强了！🎉')
    board.value.forEach(row => row.forEach(c => {
      if (c.isMine && !c.isFlagged) {
        c.isFlagged = true
        flagsUsed.value++
      }
    }))
  } else {
    // message.error('BOOM! 你输了 💥')
    if (hitR >= 0) board.value[hitR][hitC].isExploded = true
    // 显示所有雷
    board.value.forEach(row => row.forEach(c => {
      if (c.isMine) c.isRevealed = true
    }))
  }
}

const checkWin = () => {
  if (gameState.value === 'lost') return
  let safeCellsLeft = 0
  board.value.forEach(row => row.forEach(c => {
    if (!c.isMine && !c.isRevealed) safeCellsLeft++
  }))
  if (safeCellsLeft === 0) gameOver(true)
}

const startTimer = () => {
  stopTimer()
  timerInterval = setInterval(() => timer.value++, 1000)
}
const stopTimer = () => clearInterval(timerInterval)

const handleDiffChange = (val: DifficultyKey) => {
  currentDiff.value = val
  if (val === 'custom') {
    showCustomModal.value = true
  } else {
    initGame()
  }
}

const applyCustomConfig = () => {
  // 简单校验
  if (customConfig.rows < 5) customConfig.rows = 5
  if (customConfig.cols < 5) customConfig.cols = 5
  const maxMines = Math.floor(customConfig.rows * customConfig.cols * 0.9) // 最多 90% 雷
  if (customConfig.mines > maxMines) customConfig.mines = maxMines
  if (customConfig.mines < 1) customConfig.mines = 1

  showCustomModal.value = false
  initGame()
}

const getNumColor = (num: number) => {
  const colors = [
    '', 'text-[#3b82f6]', 'text-[#22c55e]', 'text-[#ef4444]',
    'text-[#a855f7]', 'text-[#eab308]', 'text-[#06b6d4]', 'text-gray-200', 'text-pink-400'
  ]
  return colors[num] || 'text-white'
}

watch(currentDiff, (v) => { if (v !== 'custom') initGame() })
onUnmounted(stopTimer)
initGame()
</script>

<template>
  <div class="h-screen w-screen flex flex-col bg-[#111827] text-slate-200 select-none overflow-hidden font-sans">

    <div class="relative h-16 flex items-center justify-between px-6 bg-[#1f2937] border-b border-slate-700 shadow-xl z-20 shrink-0">

      <div class="flex items-center gap-3 z-30">
        <n-select
          v-model:value="currentDiff"
          :options="diffOptions"
          size="small"
          :consistent-menu-width="false"
          style="width: 160px"
          @update:value="handleDiffChange"
        />
        <n-button
          v-if="currentDiff === 'custom'"
          size="small"
          secondary
          circle
          @click="showCustomModal = true"
        >
          <template #icon><n-icon :component="Settings" /></template>
        </n-button>
      </div>

      <div class="absolute inset-0 flex items-center justify-center pointer-events-none">
        <div class="flex items-center gap-6 pointer-events-auto bg-[#1f2937] px-4 rounded-xl">
          <div class="flex flex-col items-center">
            <span class="text-[10px] text-slate-500 uppercase font-bold tracking-wider">剩余地雷</span>
            <div class="font-mono text-2xl text-red-400 bg-black/40 px-3 rounded border border-red-500/20 min-w-[3.5rem] text-center shadow-[inset_0_2px_4px_rgba(0,0,0,0.5)]">
              {{ String(minesLeft).padStart(3, '0') }}
            </div>
          </div>

          <button
            class="text-4xl hover:scale-110 active:scale-95 transition-all cursor-pointer bg-black/20 rounded-full p-1 border-2 border-slate-600 hover:border-slate-400 shadow-lg"
            @click="initGame"
          >
            <n-icon v-if="gameState === 'lost'" :component="FaceDizzy" class="text-red-500" />
            <n-icon v-else-if="gameState === 'won'" :component="FaceCool" class="text-green-400" />
            <n-icon v-else :component="FaceSatisfied" class="text-yellow-400" />
          </button>

          <div class="flex flex-col items-center">
            <span class="text-[10px] text-slate-500 uppercase font-bold tracking-wider">用时</span>
            <div class="font-mono text-2xl text-red-400 bg-black/40 px-3 rounded border border-red-500/20 min-w-[3.5rem] text-center shadow-[inset_0_2px_4px_rgba(0,0,0,0.5)]">
              {{ String(timer).padStart(3, '0') }}
            </div>
          </div>
        </div>
      </div>

      <div class="w-[160px]"></div>
    </div>

    <div class="flex-1 overflow-auto flex items-center justify-center p-8 bg-[#0F172A] relative">
      <div class="absolute inset-0 opacity-5 pointer-events-none"
           style="background-image: radial-gradient(#64748b 1px, transparent 1px); background-size: 24px 24px;">
      </div>

      <div
        class="bg-[#1f2937] p-3 rounded-lg shadow-[0_20px_50px_rgba(0,0,0,0.5)] border border-slate-600 inline-block z-10"
        @contextmenu.prevent
      >
        <div
          class="grid gap-[1px] bg-[#374151] border-2 border-[#374151]"
          :style="{
            gridTemplateColumns: `repeat(${board[0]?.length || 9}, 28px)`,
            gridTemplateRows: `repeat(${board.length || 9}, 28px)`
          }"
        >
          <div
            v-for="(cell) in board.flat()"
            :key="`${cell.row}-${cell.col}`"
            class="w-7 h-7 flex items-center justify-center text-base font-bold cursor-default transition-all duration-75 select-none"
            :class="[
              // 已翻开
              cell.isRevealed
                ? (cell.isMine
                    ? (cell.isExploded ? 'bg-red-600' : 'bg-[#1f2937]')
                    : 'bg-[#e5e7eb]')

                // 未翻开 增加闪烁判断
                : (cell.isFlashing
                    ? 'bg-white/80'  // ✨ 强烈的闪烁反馈
                    : 'bg-[#4b5563] hover:bg-[#586375] shadow-[inset_1px_1px_0_rgba(255,255,255,0.15),inset_-1px_-1px_0_rgba(0,0,0,0.3)] active:shadow-none active:bg-[#e5e7eb]')
            ]"
            @click="handleLeftClick(cell.row, cell.col)"
            @contextmenu="handleRightClick($event, cell.row, cell.col)"
          >
            <template v-if="cell.isRevealed">
              <span v-if="cell.isMine">💣</span>
              <span v-else-if="cell.neighborMines > 0" :class="getNumColor(cell.neighborMines)">
                {{ cell.neighborMines }}
              </span>
            </template>

            <template v-else-if="cell.isFlagged">
              <span class="text-red-500 drop-shadow-md text-sm">🚩</span>
            </template>
          </div>
        </div>
      </div>
    </div>

    <n-modal v-model:show="showCustomModal" :mask-closable="false" transform-origin="center">
      <n-card
        class="w-[420px] bg-[#1f2937]"
        :bordered="false"
        size="huge"
        content-style="padding: 0;"
      >
        <div class="bg-gradient-to-r from-slate-800 to-slate-900 p-5 border-b border-slate-700 flex justify-between items-center">
          <div class="flex items-center gap-3">
            <div class="p-2 bg-blue-500/10 rounded-lg text-blue-400 border border-blue-500/20">
              <n-icon size="20" :component="Settings" />
            </div>
            <div>
              <div class="text-lg font-bold text-slate-100">自定义游戏</div>
              <div class="text-xs text-slate-500">无敌了孩子！</div>
            </div>
          </div>
          <n-button text class="text-slate-500 hover:text-slate-300" @click="showCustomModal = false">
            <template #icon><n-icon :component="Close" /></template>
          </n-button>
        </div>

        <div class="p-6 space-y-6">
          <div class="flex gap-4">
            <div class="flex-1 space-y-4">
              <div class="space-y-1">
                <div class="text-xs text-slate-400">高度 (行数)</div>
                <n-input-number v-model:value="customConfig.rows" :min="5" :max="50" size="small" />
              </div>
              <div class="space-y-1">
                <div class="text-xs text-slate-400">宽度 (列数)</div>
                <n-input-number v-model:value="customConfig.cols" :min="5" :max="50" size="small" />
              </div>
            </div>

            <div class="flex-1 space-y-4">
              <div class="space-y-1">
                <div class="text-xs text-slate-400">地雷总数</div>
                <n-input-number v-model:value="customConfig.mines" :min="1" size="small" />
              </div>

              <div class="bg-black/20 rounded-lg p-3 border border-slate-700/50 mt-auto">
                <div class="text-[10px] text-slate-500 uppercase tracking-wider mb-1">地雷密度</div>
                <div class="flex items-baseline gap-1">
                      <span class="text-xl font-mono font-bold" :class="parseInt(density) > 20 ? 'text-red-400' : 'text-green-400'">
                        {{ density }}
                      </span>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div class="bg-slate-800/50 p-4 border-t border-slate-700 flex justify-end gap-3">
          <n-button @click="showCustomModal = false" secondary>
            取消
          </n-button>
          <n-button type="primary" @click="applyCustomConfig" class="px-6 font-bold">
            <template #icon><n-icon :component="CheckmarkFilled" /></template>
            开始游戏
          </n-button>
        </div>
      </n-card>
    </n-modal>

  </div>
</template>

<style scoped>
/* 禁用双击选中文本 */
div {
  user-select: none;
}
</style>