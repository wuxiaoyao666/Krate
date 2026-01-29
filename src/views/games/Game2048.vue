<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { NButton, NIcon, useMessage } from 'naive-ui'
import { Restart } from '@vicons/carbon'

// === 游戏配置 ===
const GRID_SIZE = 4
const CELL_GAP = 12 // 格子间距
const CELL_SIZE = 80 // 格子大小

// === 状态定义 ===
interface Tile {
  id: number // 唯一ID，用于 Vue 的 :key 以保持动画连贯
  val: number
  r: number // 行
  c: number // 列
  isMerged?: boolean // 标记本轮是否已发生过合并
  isNew?: boolean // 标记是否是新生成的
}

const tiles = ref<Tile[]>([])
const score = ref(0)
const bestScore = ref(parseInt(localStorage.getItem('2048-best') || '0'))
const gameOver = ref(false)
const gameWon = ref(false)

let tileIdCounter = 1

// === 核心逻辑 ===

// 初始化
const initGame = () => {
  tiles.value = []
  score.value = 0
  gameOver.value = false
  gameWon.value = false
  addRandomTile()
  addRandomTile()
}

// 添加随机方块 (2 或 4)
const addRandomTile = () => {
  const emptyCells: { r: number; c: number }[] = []

  // 找出所有空位置
  for (let r = 0; r < GRID_SIZE; r++) {
    for (let c = 0; c < GRID_SIZE; c++) {
      if (!tiles.value.find((t) => t.r === r && t.c === c)) {
        emptyCells.push({ r, c })
      }
    }
  }

  if (emptyCells.length > 0) {
    const randomPos = emptyCells[Math.floor(Math.random() * emptyCells.length)]
    tiles.value.push({
      id: tileIdCounter++,
      val: Math.random() < 0.9 ? 2 : 4, // 90% 概率出 2
      r: randomPos.r,
      c: randomPos.c,
      isNew: true,
    })
  }
}

// 移动逻辑 (核心中的核心)
const move = (direction: 'up' | 'down' | 'left' | 'right') => {
  if (gameOver.value) return

  const vector = {
    up: { r: -1, c: 0 },
    down: { r: 1, c: 0 },
    left: { r: 0, c: -1 },
    right: { r: 0, c: 1 },
  }[direction]

  let moved = false
  // 清楚合并标记
  tiles.value.forEach((t) => {
    delete t.isMerged
    delete t.isNew
  })

  // 排序：移动方向靠前的先处理，防止穿透
  // 比如向上滑，那么最上面的行(r=0)先处理
  const sortedTiles = [...tiles.value].sort((a, b) => {
    if (direction === 'up') return a.r - b.r
    if (direction === 'down') return b.r - a.r
    if (direction === 'left') return a.c - b.c
    if (direction === 'right') return b.c - a.c
    return 0
  })

  // 逐个移动
  sortedTiles.forEach((tile) => {
    let { r, c } = tile

    // 尝试向该方向移动到底
    while (true) {
      const nextR = r + vector.r
      const nextC = c + vector.c

      // 边界检查
      if (nextR < 0 || nextR >= GRID_SIZE || nextC < 0 || nextC >= GRID_SIZE) break

      // 碰撞检查
      const obstacle = tiles.value.find((t) => t.r === nextR && t.c === nextC)

      if (!obstacle) {
        // 前方空荡荡，继续走
        r = nextR
        c = nextC
        moved = true
      } else if (!obstacle.isMerged && obstacle.val === tile.val) {
        // 前方有同类，且未合并过 -> 合体！
        // 1. 移除当前 tile (视觉上它滑动到了目标位置，然后消失)
        // 为了动画连贯，我们通常是移动当前 tile 到目标位，然后瞬间替换成新值的 tile
        // 但简单做法是：直接更新 obstacle 的值，删除当前 tile

        // 这里采用更细腻的做法：更新当前 tile 坐标到目标，标记为待删除
        // 实际上 Vue 只要 id 不变，坐标变了就会滑过去。
        // 我们这里简化逻辑：移动当前 tile 到目标，然后销毁它，并升级 obstacle

        // 在 Vue 列表里找到这个 obstacle 并升级
        const target = tiles.value.find((t) => t.id === obstacle.id)
        if (target) {
          target.val *= 2
          target.isMerged = true
          score.value += target.val

          // 更新最高分
          if (score.value > bestScore.value) {
            bestScore.value = score.value
            localStorage.setItem('2048-best', score.value.toString())
          }

          // 移除当前 tile
          tiles.value = tiles.value.filter((t) => t.id !== tile.id)
          moved = true
        }
        break // 这一步走完就停了
      } else {
        // 前方有障碍且无法合并
        break
      }
    }

    // 更新坐标
    tile.r = r
    tile.c = c
  })

  if (moved) {
    setTimeout(() => {
      addRandomTile()
      checkGameOver()
    }, 200) // 等动画稍微走一下再生成新的
  }
}

// 检查游戏结束
const checkGameOver = () => {
  // 1. 还有空位？没死
  if (tiles.value.length < GRID_SIZE * GRID_SIZE) return

  // 2. 没空位，检查能不能合并
  // 检查右和下即可覆盖所有相邻
  for (let r = 0; r < GRID_SIZE; r++) {
    for (let c = 0; c < GRID_SIZE; c++) {
      const current = tiles.value.find((t) => t.r === r && t.c === c)
      if (!current) continue

      const right = tiles.value.find((t) => t.r === r && t.c === c + 1)
      const down = tiles.value.find((t) => t.r === r + 1 && t.c === c)

      if (right && right.val === current.val) return
      if (down && down.val === current.val) return
    }
  }

  gameOver.value = true
}

// 键盘监听
const handleKeydown = (e: KeyboardEvent) => {
  if (gameOver.value) return

  switch (e.key) {
    case 'ArrowUp':
    case 'w':
    case 'W':
      move('up')
      break
    case 'ArrowDown':
    case 's':
    case 'S':
      move('down')
      break
    case 'ArrowLeft':
    case 'a':
    case 'A':
      move('left')
      break
    case 'ArrowRight':
    case 'd':
    case 'D':
      move('right')
      break
  }
}

// === 样式辅助 ===
const getTileStyle = (tile: Tile) => {
  const x = tile.c * (CELL_SIZE + CELL_GAP) + CELL_GAP
  const y = tile.r * (CELL_SIZE + CELL_GAP) + CELL_GAP
  return {
    transform: `translate(${x}px, ${y}px)`,
    width: `${CELL_SIZE}px`,
    height: `${CELL_SIZE}px`,
  }
}

const getTileColorClass = (val: number) => {
  const map: Record<number, string> = {
    2: 'bg-[#eee4da] text-[#776e65]',
    4: 'bg-[#ede0c8] text-[#776e65]',
    8: 'bg-[#f2b179] text-white',
    16: 'bg-[#f59563] text-white',
    32: 'bg-[#f67c5f] text-white',
    64: 'bg-[#f65e3b] text-white',
    128: 'bg-[#edcf72] text-white text-3xl',
    256: 'bg-[#edcc61] text-white text-3xl',
    512: 'bg-[#edc850] text-white text-3xl',
    1024: 'bg-[#edc53f] text-white text-2xl',
    2048: 'bg-[#edc22e] text-white text-2xl shadow-[0_0_30px_rgba(243,215,116,0.5)]',
  }
  return map[val] || 'bg-[#3c3a32] text-white text-xl'
}

onMounted(() => {
  window.addEventListener('keydown', handleKeydown)
  initGame()
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown)
})
</script>

<template>
  <div
    class="h-screen w-screen bg-[#111827] text-slate-200 flex flex-col items-center justify-center select-none font-sans"
  >
    <div class="w-[380px] flex justify-between items-start mb-6">
      <div>
        <h1 class="text-5xl font-bold text-[#eee4da]">2048</h1>
        <p class="text-slate-400 text-sm mt-1">合并方块，挑战 2048！</p>
      </div>

      <div class="flex gap-2">
        <div
          class="bg-[#bbada0] rounded-md p-2 min-w-[70px] flex flex-col items-center justify-center"
        >
          <span class="text-[#eee4da] text-xs font-bold uppercase">Score</span>
          <span class="text-white font-bold text-lg leading-none mt-1">{{ score }}</span>
        </div>
        <div
          class="bg-[#bbada0] rounded-md p-2 min-w-[70px] flex flex-col items-center justify-center"
        >
          <span class="text-[#eee4da] text-xs font-bold uppercase">Best</span>
          <span class="text-white font-bold text-lg leading-none mt-1">{{ bestScore }}</span>
        </div>
      </div>
    </div>

    <div class="w-[380px] flex justify-between items-center mb-6">
      <div class="text-slate-500 text-xs">
        使用 <span class="font-bold text-slate-300">方向键</span> 或
        <span class="font-bold text-slate-300">WASD</span> 移动
      </div>
      <n-button type="primary" color="#8f7a66" @click="initGame">
        <template #icon><n-icon :component="Restart" /></template>
        新游戏
      </n-button>
    </div>

    <div
      class="relative bg-[#bbada0] rounded-lg p-[12px]"
      :style="{
        width: `${GRID_SIZE * (CELL_SIZE + CELL_GAP) + CELL_GAP}px`,
        height: `${GRID_SIZE * (CELL_SIZE + CELL_GAP) + CELL_GAP}px`,
      }"
    >
      <div
        v-for="i in 16"
        :key="`bg-${i}`"
        class="absolute bg-[#cdc1b4] rounded opacity-30"
        :style="{
          width: `${CELL_SIZE}px`,
          height: `${CELL_SIZE}px`,
          left: `${((i - 1) % 4) * (CELL_SIZE + CELL_GAP) + CELL_GAP}px`,
          top: `${Math.floor((i - 1) / 4) * (CELL_SIZE + CELL_GAP) + CELL_GAP}px`,
        }"
      ></div>

      <transition-group name="tile">
        <div
          v-for="tile in tiles"
          :key="tile.id"
          class="absolute rounded flex items-center justify-center font-bold text-4xl shadow-sm transition-transform duration-150 ease-in-out"
          :class="[
            getTileColorClass(tile.val),
            { 'tile-new': tile.isNew, 'tile-merged': tile.isMerged },
          ]"
          :style="getTileStyle(tile)"
        >
          {{ tile.val }}
        </div>
      </transition-group>

      <div
        v-if="gameOver"
        class="absolute inset-0 bg-[#eee4da]/70 flex flex-col items-center justify-center rounded-lg z-10 backdrop-blur-sm animate-fade-in"
      >
        <h2 class="text-[#776e65] text-4xl font-bold mb-4">Game Over!</h2>
        <n-button size="large" type="primary" color="#8f7a66" @click="initGame">
          再试一次
        </n-button>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 新生成的方块动画 */
.tile-new {
  animation: appear 0.2s ease-in-out;
}

/* 合并时的方块动画 */
.tile-merged {
  animation: pop 0.2s ease-in-out;
}

@keyframes appear {
  0% {
    opacity: 0;
    transform: scale(0);
  }
  100% {
    opacity: 1;
    transform: scale(1);
  }
}

@keyframes pop {
  0% {
    transform: scale(1);
  }
  50% {
    transform: scale(1.2);
  }
  100% {
    transform: scale(1);
  }
}

.animate-fade-in {
  animation: fadeIn 0.5s ease-in-out;
}

@keyframes fadeIn {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

/* 确保动画过程中位置也是绝对定位 */
.tile-move {
  transition: transform 0.15s ease-in-out;
}
</style>
