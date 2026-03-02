<script setup lang="ts">
import { emitTo } from '@tauri-apps/api/event'
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { NGrid, NGi, NCard, NButton, NIcon, useMessage } from 'naive-ui'
import { Flash, PlayFilled, TableSplit, GameConsole, Grid } from '@vicons/carbon'
import { COW_PUZZLE_REFRESH_EVENT } from './cowPuzzleEngine'

const message = useMessage()

const games = [
  {
    id: 'minesweeper',
    title: '扫雷',
    desc: '经典逻辑游戏，挑战你的推理能力',
    icon: Flash,
    color: 'text-red-400',
    route: '/game/minesweeper',
    width: 1200,
    height: 900,
  },
  {
    id: '2048',
    title: '2048',
    desc: '数字合成的艺术，根本停不下来',
    icon: TableSplit,
    color: 'text-orange-400',
    route: '/game/2048',
    width: 520,
    height: 760,
  },
  {
    id: 'breakout',
    title: '霓虹打砖块',
    desc: '击碎束缚，释放压力',
    icon: GameConsole,
    color: 'text-emerald-400',
    route: '/game/breakout',
    width: 1200,
    height: 900,
  },
  {
    id: 'cow-puzzle',
    title: '逻辑放牛',
    desc: '随机生成的颜色逻辑题，右键落牛，放错即结束',
    icon: Grid,
    color: 'text-indigo-400',
    route: '/game/cow-puzzle',
    width: 1160,
    height: 840,
  },
]

const openGameWindow = async (game: any) => {
  const label = `game-${game.id}` // 窗口唯一标识

  // 1. 检查是否已打开
  const existingWin = await WebviewWindow.getByLabel(label)
  if (existingWin) {
    await existingWin.setFocus()
    if (game.id === 'cow-puzzle') {
      await emitTo(label, COW_PUZZLE_REFRESH_EVENT)
    }
    return
  }

  // 2. 创建新窗口
  const webview = new WebviewWindow(label, {
    url: `${game.route}`,
    title: game.title,
    width: game.width,
    height: game.height,
    resizable: false,
    center: true,
  })

  webview.once('tauri://error', (e) => {
    message.error('无法打开窗口: ' + JSON.stringify(e))
  })
}
</script>

<template>
  <div class="p-8 max-w-6xl mx-auto">
    <div class="mb-8">
      <h1 class="text-3xl font-bold text-slate-100 mb-2">🎮 游戏大厅</h1>
      <p class="text-slate-400">点击下方游戏，将在独立窗口启动。</p>
    </div>

    <n-grid x-gap="24" y-gap="24" cols="1 s:2 m:3" responsive="screen">
      <n-gi v-for="game in games" :key="game.id">
        <n-card
          class="bg-[#1E293B] border-slate-700 hover:-translate-y-1 transition-all duration-300"
        >
          <div class="flex flex-col gap-4">
            <div class="flex items-center gap-3">
              <n-icon :size="32" :component="game.icon" :class="game.color" />
              <span class="text-lg font-bold text-slate-100">{{ game.title }}</span>
            </div>
            <p class="text-slate-400 text-sm h-10">{{ game.desc }}</p>
            <n-button block type="primary" secondary @click="openGameWindow(game)">
              <template #icon><n-icon :component="PlayFilled" /></template>
              启动
            </n-button>
          </div>
        </n-card>
      </n-gi>
    </n-grid>
  </div>
</template>
