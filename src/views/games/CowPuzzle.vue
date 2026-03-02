<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import { listen } from '@tauri-apps/api/event'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { NButton, NIcon, NSpin, useMessage } from 'naive-ui'
import { Restart } from '@vicons/carbon'

import {
  COW_PUZZLE_REFRESH_EVENT,
  cowPuzzleDifficultyOptions,
  generateCowPuzzle,
} from './cowPuzzleEngine'
import type { DifficultyKey, PuzzleDefinition } from './cowPuzzleEngine'

type CellMark = 'none' | 'cross' | 'auto' | 'cow' | 'wrong'
type GameStatus = 'idle' | 'generating' | 'playing' | 'won' | 'lost'

const message = useMessage()

const currentDifficulty = ref<DifficultyKey>('standard')
const currentPuzzle = ref<PuzzleDefinition | null>(null)
const marks = ref<CellMark[][]>([])
const gameStatus = ref<GameStatus>('idle')
const elapsedSeconds = ref(0)
const wrongCell = ref<{ row: number; col: number } | null>(null)

const drag = reactive({
  active: false,
  lastKey: '',
})

let timerHandle: number | null = null
let generationToken = 0
let refreshUnlisten: UnlistenFn | null = null

function createMarks(size: number) {
  return Array.from({ length: size }, () => Array<CellMark>(size).fill('none'))
}

function stopTimer() {
  if (timerHandle !== null) {
    window.clearInterval(timerHandle)
    timerHandle = null
  }
}

function startTimer() {
  stopTimer()
  timerHandle = window.setInterval(() => {
    elapsedSeconds.value++
  }, 1000)
}

function stopDrag() {
  drag.active = false
  drag.lastKey = ''
}

function getRegionColor(row: number, col: number) {
  const puzzle = currentPuzzle.value
  if (!puzzle) return '#cbd5e1'
  return puzzle.palette[puzzle.regions[row][col]]
}

function isSolutionCell(row: number, col: number) {
  return currentPuzzle.value?.solutionCols[row] === col
}

function getMark(row: number, col: number) {
  return marks.value[row]?.[col] ?? 'none'
}

function applyAutoCrosses(row: number, col: number) {
  const puzzle = currentPuzzle.value
  if (!puzzle) return

  const size = puzzle.size
  const regionId = puzzle.regions[row][col]

  const setAuto = (targetRow: number, targetCol: number) => {
    const current = marks.value[targetRow][targetCol]
    if (current === 'cow' || current === 'wrong') return
    if (current === 'none' || current === 'cross') {
      marks.value[targetRow][targetCol] = 'auto'
    }
  }

  for (let index = 0; index < size; index++) {
    if (index !== col) setAuto(row, index)
    if (index !== row) setAuto(index, col)
  }

  for (let regionRow = 0; regionRow < size; regionRow++) {
    for (let regionCol = 0; regionCol < size; regionCol++) {
      if (regionRow === row && regionCol === col) continue
      if (puzzle.regions[regionRow][regionCol] === regionId) {
        setAuto(regionRow, regionCol)
      }
    }
  }

  for (let dr = -1; dr <= 1; dr++) {
    for (let dc = -1; dc <= 1; dc++) {
      if (dr === 0 && dc === 0) continue
      const nextRow = row + dr
      const nextCol = col + dc
      if (nextRow < 0 || nextRow >= size || nextCol < 0 || nextCol >= size) continue
      setAuto(nextRow, nextCol)
    }
  }
}

function finishWithLoss(row: number, col: number) {
  wrongCell.value = { row, col }
  marks.value[row][col] = 'wrong'
  gameStatus.value = 'lost'
  stopTimer()
  stopDrag()
  message.error('这格不是正确位置，棋盘已揭示。')
}

function finishWithWin() {
  gameStatus.value = 'won'
  stopTimer()
  stopDrag()
  message.success('全部小牛都找到了。')
}

function placeCow(row: number, col: number) {
  if (gameStatus.value !== 'playing' || !currentPuzzle.value) return

  const mark = getMark(row, col)
  if (mark === 'cow' || mark === 'auto' || mark === 'wrong') return

  if (!isSolutionCell(row, col)) {
    finishWithLoss(row, col)
    return
  }

  marks.value[row][col] = 'cow'
  applyAutoCrosses(row, col)

  if (foundCowCount.value === currentPuzzle.value.size) {
    finishWithWin()
  }
}

function markCross(row: number, col: number) {
  if (getMark(row, col) !== 'none') return
  marks.value[row][col] = 'cross'
}

function clearManualCross(row: number, col: number) {
  if (getMark(row, col) === 'cross') {
    marks.value[row][col] = 'none'
  }
}

function handleCellPointerDown(row: number, col: number, event: PointerEvent) {
  if (gameStatus.value !== 'playing') return
  if (event.button !== 0) return
  event.preventDefault()

  const mark = getMark(row, col)
  if (mark === 'cow' || mark === 'auto' || mark === 'wrong') {
    stopDrag()
    return
  }

  if (mark === 'cross') {
    clearManualCross(row, col)
    stopDrag()
    return
  }

  markCross(row, col)
  drag.active = true
  drag.lastKey = `${row}-${col}`
}

function handleCellPointerEnter(row: number, col: number, event: PointerEvent) {
  if (!drag.active) return
  if ((event.buttons & 1) === 0) {
    stopDrag()
    return
  }

  const key = `${row}-${col}`
  if (drag.lastKey === key) return
  drag.lastKey = key

  if (getMark(row, col) === 'none') {
    markCross(row, col)
  }
}

function handleCellContextMenu(row: number, col: number, event: MouseEvent) {
  event.preventDefault()
  placeCow(row, col)
}

function formatDuration(totalSeconds: number) {
  const minutes = Math.floor(totalSeconds / 60)
  const seconds = totalSeconds % 60
  return `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`
}

async function startNewGame(nextDifficulty = currentDifficulty.value) {
  const token = ++generationToken
  currentDifficulty.value = nextDifficulty
  gameStatus.value = 'generating'
  wrongCell.value = null
  stopTimer()
  stopDrag()
  elapsedSeconds.value = 0

  await nextTick()

  try {
    const puzzle = await new Promise<PuzzleDefinition>((resolve) => {
      window.setTimeout(() => resolve(generateCowPuzzle(nextDifficulty)), 16)
    })

    if (token !== generationToken) return

    currentPuzzle.value = puzzle
    marks.value = createMarks(puzzle.size)
    wrongCell.value = null
    gameStatus.value = 'playing'
    startTimer()
  } catch (error) {
    if (token !== generationToken) return
    gameStatus.value = 'idle'
    message.error(error instanceof Error ? error.message : '生成谜题失败')
  }
}

const size = computed(() => currentPuzzle.value?.size ?? 8)
const boardCells = computed(() => {
  if (!currentPuzzle.value) return []
  return Array.from({ length: currentPuzzle.value.size }, (_, row) =>
    Array.from({ length: currentPuzzle.value!.size }, (_, col) => ({
      row,
      col,
      key: `${row}-${col}`,
    }))
  )
})
const foundCowCount = computed(() =>
  marks.value.reduce((sum, row) => sum + row.filter((cell) => cell === 'cow').length, 0)
)
const remainingCows = computed(() => (currentPuzzle.value ? currentPuzzle.value.size - foundCowCount.value : 0))
const revealAllCows = computed(() => gameStatus.value === 'lost')
const logicMeter = computed(() => {
  if (!currentPuzzle.value) return 24
  return Math.min(100, Math.max(18, currentPuzzle.value.rating.score * 1.8))
})
const openingHint = computed(() => {
  const opening = currentPuzzle.value?.rating.openingForcedPlacements ?? 0
  if (opening <= 1) return '起手线索少'
  if (opening <= 3) return '起手线索适中'
  return '起手线索偏多'
})
const chainHint = computed(() => {
  const waves = currentPuzzle.value?.rating.propagationWaves ?? 0
  if (waves <= 1) return '联动较短'
  if (waves <= 3) return '联动中等'
  return '联动较深'
})
const difficultyCopy = computed(() => {
  if (!currentPuzzle.value) return '纯逻辑可解'
  return `${openingHint.value}，${chainHint.value}`
})
const statusTitle = computed(() => {
  if (gameStatus.value === 'won') return '牛群归位'
  if (gameStatus.value === 'lost') return '踩错位置'
  if (gameStatus.value === 'generating') return '正在排局'
  return '逻辑放牛'
})
const statusSubtitle = computed(() => {
  if (!currentPuzzle.value) return '生成后开始挑战'
  if (gameStatus.value === 'won') {
    return `用时 ${formatDuration(elapsedSeconds.value)}，题面力度 ${currentPuzzle.value.rating.score}`
  }
  if (gameStatus.value === 'lost') return '右键必须落在真实答案上，放错直接结束'
  if (gameStatus.value === 'generating') return '正在筛掉过水的随机盘面'
  return `${currentPuzzle.value.size} 头牛，颜色 / 行 / 列各一，且不能相邻`
})
const boardStyle = computed(() => ({
  gridTemplateColumns: `repeat(${size.value}, minmax(0, 1fr))`,
  width: `min(calc(100vw - 430px), calc(100vh - 240px), ${size.value === 6 ? 410 : 510}px)`,
}))

function cellClass(row: number, col: number) {
  const mark = getMark(row, col)
  return {
    'cell--manual-cross': mark === 'cross',
    'cell--auto-cross': mark === 'auto',
    'cell--cow': mark === 'cow',
    'cell--wrong': mark === 'wrong',
    'cell--revealed-cow': revealAllCows.value && isSolutionCell(row, col) && mark !== 'cow',
    'cell--mistake': wrongCell.value?.row === row && wrongCell.value?.col === col,
  }
}

function showCow(row: number, col: number) {
  return getMark(row, col) === 'cow' || (revealAllCows.value && isSolutionCell(row, col))
}

onMounted(async () => {
  window.addEventListener('pointerup', stopDrag)
  window.addEventListener('blur', stopDrag)

  try {
    refreshUnlisten = await listen(COW_PUZZLE_REFRESH_EVENT, async () => {
      await startNewGame()
    })
  } catch {
    refreshUnlisten = null
  }

  await startNewGame(currentDifficulty.value)
})

onBeforeUnmount(() => {
  stopTimer()
  stopDrag()
  window.removeEventListener('pointerup', stopDrag)
  window.removeEventListener('blur', stopDrag)
  refreshUnlisten?.()
  refreshUnlisten = null
})
</script>

<template>
  <div class="cow-page" @contextmenu.prevent>
    <div class="cow-page__halo cow-page__halo--mint"></div>
    <div class="cow-page__halo cow-page__halo--sun"></div>

    <div class="cow-shell">
      <section class="hero-card">
        <div class="hero-copy">
          <p class="eyebrow">牧场谜题</p>
          <h1>{{ statusTitle }}</h1>
          <p class="hero-subtitle">{{ statusSubtitle }}</p>
        </div>

        <div class="hero-actions">
          <div class="difficulty-switch">
            <button
              v-for="option in cowPuzzleDifficultyOptions"
              :key="option.value"
              type="button"
              class="difficulty-chip"
              :class="{ 'difficulty-chip--active': currentDifficulty === option.value }"
              :disabled="gameStatus === 'generating'"
              @click="startNewGame(option.value)"
            >
              {{ option.label }}
            </button>
          </div>

          <n-button
            type="primary"
            strong
            secondary
            class="new-game-button"
            :disabled="gameStatus === 'generating'"
            @click="startNewGame()"
          >
            <template #icon>
              <n-icon :component="Restart" />
            </template>
            随机新局
          </n-button>
        </div>
      </section>

      <div class="cow-layout">
        <section class="board-card">
          <div class="rules-strip">
            <div class="rule-pill">每种颜色恰好 1 头牛</div>
            <div class="rule-pill">每一行、每一列都恰好 1 头牛</div>
            <div class="rule-pill">牛与牛之间不能相邻</div>
          </div>

          <div class="board-frame">
            <div v-if="currentPuzzle" class="board-grid" :style="boardStyle">
              <template v-for="rowCells in boardCells" :key="rowCells[0]?.row ?? 0">
                <button
                  v-for="cell in rowCells"
                  :key="cell.key"
                  type="button"
                  class="board-cell"
                  :class="cellClass(cell.row, cell.col)"
                  :style="{
                    backgroundColor: getRegionColor(cell.row, cell.col),
                    animationDelay: `${(cell.row + cell.col) * 18}ms`,
                  }"
                  @pointerdown="handleCellPointerDown(cell.row, cell.col, $event)"
                  @pointerenter="handleCellPointerEnter(cell.row, cell.col, $event)"
                  @contextmenu="handleCellContextMenu(cell.row, cell.col, $event)"
                >
                  <span
                    v-if="showCow(cell.row, cell.col)"
                    class="cell-cow"
                    :class="{ 'cell-cow--ghost': getMark(cell.row, cell.col) !== 'cow' }"
                  >
                    🐮
                  </span>

                  <span
                    v-if="getMark(cell.row, cell.col) === 'cross' || getMark(cell.row, cell.col) === 'auto'"
                    class="cell-cross"
                    :class="{ 'cell-cross--auto': getMark(cell.row, cell.col) === 'auto' }"
                  >
                    <i></i>
                    <i></i>
                  </span>

                  <span v-if="getMark(cell.row, cell.col) === 'wrong'" class="cell-burst">!</span>
                </button>
              </template>
            </div>

            <div v-if="gameStatus === 'generating'" class="board-overlay">
              <n-spin size="large" />
              <p>正在准备新的一局</p>
            </div>
          </div>
        </section>

        <aside class="side-panel">
          <div class="stat-card stat-card--primary">
            <div>
              <p class="stat-label">剩余小牛</p>
              <p class="stat-value">{{ remainingCows }}</p>
            </div>
            <p class="stat-footnote">已找到 {{ foundCowCount }} / {{ currentPuzzle?.size ?? 0 }}</p>
          </div>

          <div class="stat-grid">
            <div class="stat-card">
              <p class="stat-label">计时</p>
              <p class="stat-value stat-value--sm">{{ formatDuration(elapsedSeconds) }}</p>
            </div>
            <div class="stat-card">
              <p class="stat-label">题面力度</p>
              <p class="stat-value stat-value--sm">{{ currentPuzzle?.rating.score ?? '--' }}</p>
            </div>
          </div>

          <div class="stat-card">
            <div class="meter-header">
              <p class="stat-label">难度档位</p>
              <span>{{ currentPuzzle?.difficultyLabel ?? '标准' }}</span>
            </div>
            <div class="logic-meter">
              <div class="logic-meter__fill" :style="{ width: `${logicMeter}%` }"></div>
            </div>
            <p class="meter-copy">{{ difficultyCopy }}</p>
          </div>

          <div class="tips-card">
            <p class="tips-title">交互说明</p>
            <ul>
              <li>左键：标记当前位置为空地</li>
              <li>左键拖动：连续批量打叉</li>
              <li>右键：在当前位置放牛，放错直接结束</li>
              <li>放对一头牛后，会自动划掉同排同列同色和相邻格</li>
            </ul>
          </div>

          <div class="footer-note">
            <span>局号 {{ currentPuzzle?.seed ?? '----' }}</span>
            <span>每次打开可生成新局</span>
          </div>
        </aside>
      </div>
    </div>
  </div>
</template>

<style scoped>
.cow-page {
  position: relative;
  height: 100vh;
  overflow: hidden;
  background:
    radial-gradient(circle at top left, rgba(255, 247, 230, 0.96), rgba(246, 241, 233, 0.92) 42%, rgba(228, 232, 236, 0.9) 100%),
    linear-gradient(145deg, #f8f2e7, #dfe7ea);
  color: #31424f;
  font-family: 'Avenir Next', 'PingFang SC', 'Microsoft YaHei', sans-serif;
}

.cow-page__halo {
  position: absolute;
  border-radius: 999px;
  filter: blur(22px);
  opacity: 0.55;
  pointer-events: none;
}

.cow-page__halo--mint {
  width: 280px;
  height: 280px;
  top: 64px;
  right: 72px;
  background: rgba(141, 216, 191, 0.54);
}

.cow-page__halo--sun {
  width: 240px;
  height: 240px;
  left: 48px;
  bottom: 64px;
  background: rgba(245, 196, 128, 0.4);
}

.cow-shell {
  position: relative;
  z-index: 1;
  max-width: 1220px;
  margin: 0 auto;
  height: 100%;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  padding: 16px 18px 14px;
}

.hero-card,
.board-card,
.stat-card,
.tips-card {
  background: rgba(255, 255, 255, 0.74);
  border: 1px solid rgba(255, 255, 255, 0.92);
  box-shadow:
    0 24px 48px rgba(122, 139, 160, 0.16),
    inset 0 1px 0 rgba(255, 255, 255, 0.7);
  backdrop-filter: blur(20px);
}

.hero-card {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 24px;
  padding: 20px 22px;
  border-radius: 24px;
  flex: 0 0 auto;
}

.eyebrow {
  margin: 0 0 6px;
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0.18em;
  text-transform: uppercase;
  color: #7f8c97;
}

.hero-copy h1 {
  margin: 0;
  font-size: clamp(24px, 3vw, 34px);
  line-height: 1;
  letter-spacing: -0.04em;
  color: #21343f;
}

.hero-subtitle {
  margin: 8px 0 0;
  font-size: 14px;
  color: #60717b;
}

.hero-actions {
  display: flex;
  align-items: center;
  gap: 16px;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.difficulty-switch {
  display: inline-flex;
  gap: 8px;
  padding: 8px;
  border-radius: 999px;
  background: rgba(232, 236, 240, 0.9);
  box-shadow: inset 0 1px 2px rgba(114, 131, 149, 0.12);
}

.difficulty-chip {
  border: 0;
  border-radius: 999px;
  padding: 8px 14px;
  background: transparent;
  color: #5b6d77;
  font-size: 13px;
  font-weight: 700;
  cursor: pointer;
  transition:
    transform 0.18s ease,
    background-color 0.18s ease,
    color 0.18s ease,
    box-shadow 0.18s ease;
}

.difficulty-chip:hover:not(:disabled) {
  transform: translateY(-1px);
  color: #223740;
}

.difficulty-chip--active {
  background: linear-gradient(135deg, #21343f, #45606b);
  color: #f8fbfd;
  box-shadow: 0 12px 20px rgba(56, 78, 92, 0.28);
}

.difficulty-chip:disabled {
  opacity: 0.55;
  cursor: wait;
}

.new-game-button {
  min-width: 116px;
}

.cow-layout {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(250px, 300px);
  gap: 16px;
  margin-top: 16px;
  min-height: 0;
  flex: 1 1 auto;
}

.board-card {
  border-radius: 24px;
  padding: 16px;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.rules-strip {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
  flex: 0 0 auto;
}

.rule-pill {
  padding: 11px 12px;
  border-radius: 16px;
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.96), rgba(241, 244, 247, 0.9));
  color: #576972;
  font-size: 12px;
  font-weight: 700;
  text-align: center;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.72);
}

.board-frame {
  position: relative;
  margin-top: 14px;
  display: flex;
  justify-content: center;
  align-items: center;
  padding: 12px;
  border-radius: 22px;
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.72), rgba(232, 238, 242, 0.9)),
    linear-gradient(145deg, rgba(205, 216, 224, 0.6), rgba(255, 255, 255, 0.4));
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.88),
    0 18px 28px rgba(135, 151, 166, 0.16);
  flex: 1 1 auto;
  min-height: 0;
}

.board-grid {
  display: grid;
  gap: 7px;
  aspect-ratio: 1 / 1;
  width: min(100%, var(--board-side, 510px));
}

.board-cell {
  position: relative;
  border: 0;
  border-radius: 14px;
  cursor: pointer;
  overflow: hidden;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.45);
  transition:
    transform 0.14s ease,
    filter 0.18s ease,
    box-shadow 0.18s ease,
    background-color 0.18s ease;
  animation: cell-rise 0.36s ease both;
}

.board-cell:hover {
  transform: translateY(-1px) scale(1.01);
  filter: saturate(1.04);
}

.board-cell:active {
  transform: scale(0.98);
}

.board-cell::after {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: inherit;
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.14), transparent 56%);
  pointer-events: none;
}

.cell--cow {
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.52),
    0 10px 24px rgba(63, 74, 83, 0.18);
}

.cell--revealed-cow {
  filter: saturate(0.8) brightness(1.04);
}

.cell--wrong {
  background: linear-gradient(145deg, #f97066, #ee695b) !important;
  box-shadow:
    0 0 0 2px rgba(255, 255, 255, 0.72),
    0 16px 32px rgba(226, 101, 87, 0.34);
}

.cell--mistake {
  animation:
    cell-rise 0.36s ease both,
    shake 0.42s ease;
}

.cell-cow {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  font-size: clamp(22px, 2vw, 30px);
  text-shadow: 0 8px 12px rgba(255, 255, 255, 0.28);
  transform: translateY(1px);
  z-index: 2;
  animation: cow-pop 0.36s cubic-bezier(0.18, 0.89, 0.32, 1.28);
}

.cell-cow--ghost {
  opacity: 0.48;
  filter: grayscale(0.14);
  animation: cow-reveal 0.4s ease;
}

.cell-cross {
  position: absolute;
  inset: 0;
  z-index: 2;
  display: grid;
  place-items: center;
  animation: cross-fade 0.18s ease;
}

.cell-cross i {
  position: absolute;
  inset: 50% auto auto 50%;
  width: 62%;
  height: 4px;
  border-radius: 999px;
  background: linear-gradient(90deg, transparent 0%, rgba(34, 49, 60, 0.78) 18%, rgba(34, 49, 60, 0.92) 50%, rgba(34, 49, 60, 0.78) 82%, transparent 100%);
  transform-origin: center;
  box-shadow:
    0 1px 6px rgba(32, 44, 54, 0.12),
    0 0 0 1px rgba(255, 255, 255, 0.08);
}

.cell-cross i:first-child {
  transform: translate(-50%, -50%) rotate(43deg);
}

.cell-cross i:last-child {
  transform: translate(-50%, -50%) rotate(-43deg);
}

.cell-cross--auto i {
  width: 48%;
  height: 3px;
  background: linear-gradient(90deg, transparent 0%, rgba(53, 85, 97, 0.16) 20%, rgba(53, 85, 97, 0.3) 50%, rgba(53, 85, 97, 0.16) 80%, transparent 100%);
  box-shadow: none;
}

.cell-burst {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  z-index: 3;
  font-size: clamp(24px, 2.2vw, 34px);
  font-weight: 900;
  color: #fff3ee;
  animation: burst-pop 0.28s ease-out;
}

.board-overlay {
  position: absolute;
  inset: 16px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 14px;
  border-radius: 24px;
  background: rgba(247, 249, 251, 0.76);
  backdrop-filter: blur(8px);
  color: #4c626f;
  font-weight: 700;
}

.side-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
}

.stat-card,
.tips-card {
  border-radius: 20px;
  padding: 16px;
}

.stat-card--primary {
  background: linear-gradient(145deg, rgba(34, 52, 63, 0.94), rgba(71, 104, 116, 0.88));
  color: #f7fafc;
}

.stat-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.stat-label {
  margin: 0;
  font-size: 13px;
  font-weight: 700;
  letter-spacing: 0.02em;
  color: inherit;
  opacity: 0.72;
}

.stat-value {
  margin: 8px 0 0;
  font-size: 38px;
  font-weight: 800;
  line-height: 1;
}

.stat-value--sm {
  font-size: 24px;
}

.stat-footnote {
  margin: 12px 0 0;
  font-size: 13px;
  opacity: 0.76;
}

.meter-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 14px;
  color: #556873;
  font-size: 13px;
  font-weight: 700;
}

.logic-meter {
  height: 14px;
  border-radius: 999px;
  background: rgba(207, 217, 224, 0.76);
  overflow: hidden;
}

.logic-meter__fill {
  height: 100%;
  border-radius: inherit;
  background: linear-gradient(90deg, #6f8fbf, #72c3b0, #e8bd62);
  box-shadow: 0 10px 18px rgba(111, 143, 191, 0.28);
  transition: width 0.3s ease;
}

.meter-copy {
  margin: 10px 0 0;
  font-size: 12px;
  line-height: 1.55;
  color: #60717b;
}

.tips-title {
  margin: 0 0 12px;
  font-size: 15px;
  font-weight: 800;
  color: #2d434d;
}

.tips-card ul {
  margin: 0;
  padding-left: 18px;
  color: #5f727d;
  line-height: 1.65;
  font-size: 12px;
}

.footer-note {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  padding: 0 4px;
  color: #6d808b;
  font-size: 12px;
  letter-spacing: 0.02em;
  margin-top: auto;
}

@keyframes cell-rise {
  from {
    opacity: 0;
    transform: translateY(8px) scale(0.96);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}

@keyframes cow-pop {
  0% {
    opacity: 0;
    transform: scale(0.52) rotate(-8deg);
  }
  65% {
    opacity: 1;
    transform: scale(1.14) rotate(3deg);
  }
  100% {
    opacity: 1;
    transform: scale(1) rotate(0deg);
  }
}

@keyframes cow-reveal {
  from {
    opacity: 0;
    transform: scale(0.9);
  }
  to {
    opacity: 0.48;
    transform: scale(1);
  }
}

@keyframes cross-fade {
  from {
    opacity: 0;
    transform: scale(0.85);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}

@keyframes burst-pop {
  0% {
    opacity: 0;
    transform: scale(0.3);
  }
  75% {
    opacity: 1;
    transform: scale(1.18);
  }
  100% {
    opacity: 1;
    transform: scale(1);
  }
}

@keyframes shake {
  0%,
  100% {
    transform: translateX(0);
  }
  20% {
    transform: translateX(-4px);
  }
  40% {
    transform: translateX(4px);
  }
  60% {
    transform: translateX(-3px);
  }
  80% {
    transform: translateX(3px);
  }
}

@media (max-width: 980px) {
  .cow-shell {
    overflow-y: auto;
    padding: 16px;
  }

  .hero-card {
    flex-direction: column;
    align-items: flex-start;
  }

  .hero-actions {
    width: 100%;
    justify-content: space-between;
  }

  .cow-layout {
    grid-template-columns: 1fr;
    overflow: visible;
  }

  .footer-note {
    padding-bottom: 10px;
  }
}

@media (max-width: 720px) {
  .cow-page {
    height: auto;
    min-height: 100vh;
    overflow: auto;
  }

  .rules-strip {
    grid-template-columns: 1fr;
  }

  .difficulty-switch {
    width: 100%;
    justify-content: space-between;
  }

  .difficulty-chip {
    flex: 1;
    text-align: center;
  }

  .hero-actions {
    align-items: stretch;
  }

  .new-game-button {
    width: 100%;
  }

  .stat-grid {
    grid-template-columns: 1fr;
  }

  .footer-note {
    flex-direction: column;
  }
}
</style>
