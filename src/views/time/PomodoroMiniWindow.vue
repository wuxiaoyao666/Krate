<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import {
  Cafe,
  ChartLineData,
  Checkmark,
  Maximize,
  Pause,
  Play,
  Reset,
  Timer,
} from '@vicons/carbon'
import {
  emitPomodoroMiniAction,
  listenPomodoroRuntimeSnapshot,
  readPomodoroRuntimeSnapshot,
  restoreMainWindow,
} from '@/hooks/usePomodoroMiniBridge'

const snapshot = ref(readPomodoroRuntimeSnapshot())

let unlistenRuntime: UnlistenFn | null = null

const titleText = computed(() => {
  if (snapshot.value.mode === 'break') return '正在休息'
  return snapshot.value.currentTaskTitle || '自由专注'
})

const showBreakAction = computed(() => snapshot.value.mode !== 'break')
const showCompleteAction = computed(() => snapshot.value.hasCurrentTask && snapshot.value.mode !== 'break')
const showModeToggleAction = computed(() => !snapshot.value.hasCurrentTask && snapshot.value.mode !== 'break')

const modeToggleTitle = computed(() =>
  snapshot.value.mode === 'timer' ? '切换到正向计时' : '切换到倒计时',
)

async function handlePrimaryAction() {
  if (snapshot.value.mode === 'break') {
    await emitPomodoroMiniAction({ type: 'endBreak' })
    return
  }

  await emitPomodoroMiniAction({ type: 'toggleTimer' })
}

async function handleBreakAction() {
  await emitPomodoroMiniAction({ type: 'startBreak' })
}

async function handleCompleteAction() {
  await emitPomodoroMiniAction({ type: 'completeTask' })
}

async function handleModeToggle() {
  await emitPomodoroMiniAction({ type: 'toggleMode' })
}

async function handleReset() {
  await emitPomodoroMiniAction({ type: 'resetTimer' })
}

async function expandToMain() {
  try {
    await restoreMainWindow()
  } catch (error) {
    console.error('恢复主窗口失败:', error)
  }

  try {
    await emitPomodoroMiniAction({ type: 'restoreMain' })
  } catch {
    // ignore
  }

  try {
    await getCurrentWindow().close()
  } catch (error) {
    console.error('关闭 mini 失败，尝试销毁窗口:', error)
    try {
      await getCurrentWindow().destroy()
    } catch (destroyError) {
      console.error('销毁 mini 失败:', destroyError)
    }
  }
}

async function startWindowDrag(event: MouseEvent) {
  const target = event.target as HTMLElement | null
  if (!target) return
  if (target.closest('button')) return
  await getCurrentWindow().startDragging()
}

onMounted(async () => {
  snapshot.value = readPomodoroRuntimeSnapshot()
  unlistenRuntime = await listenPomodoroRuntimeSnapshot((nextSnapshot) => {
    snapshot.value = nextSnapshot
  })
})

onUnmounted(() => {
  if (unlistenRuntime) {
    unlistenRuntime()
    unlistenRuntime = null
  }

})
</script>

<template>
  <div class="h-screen w-screen overflow-hidden bg-transparent text-slate-200 select-none p-[2px]">
    <div
      @mousedown.left.capture="startWindowDrag"
      class="group flex h-full w-full flex-col items-center justify-center relative border border-white/10 shadow-2xl"
      :class="'bg-slate-950 rounded-2xl overflow-hidden'"
    >
      <div class="z-10 flex flex-col items-center gap-1 max-w-[90%] pointer-events-none select-none">
        <div
          class="text-xl md:text-2xl font-bold text-center leading-tight truncate w-full"
          :class="snapshot.mode === 'break' ? 'text-indigo-400' : 'text-white'"
        >
          {{ titleText }}
        </div>

        <div class="font-mono text-sm font-medium transition-colors duration-300" :class="snapshot.themeClass">
          {{ snapshot.displayTime }}
        </div>
      </div>

      <div class="w-full h-1 bg-slate-800 absolute bottom-0 left-0 z-20">
        <div
          class="h-full transition-all duration-700"
          :class="snapshot.progressBarClass"
          :style="{ width: `${snapshot.progressPercentage}%` }"
        ></div>
      </div>

      <div
        class="absolute inset-0 flex items-center justify-center gap-2 z-30 opacity-0 group-hover:opacity-100 bg-slate-950/85 backdrop-blur-sm transition-all duration-200 pointer-events-none"
      >
        <button
          @click="handlePrimaryAction"
          class="p-2 rounded-full bg-slate-800 hover:bg-slate-700 text-white cursor-pointer pointer-events-auto transition-colors"
          :title="snapshot.mode === 'break' ? '结束休息' : snapshot.isRunning ? '暂停' : '开始'"
        >
          <component :is="snapshot.mode === 'break' ? Play : snapshot.isRunning ? Pause : Play" class="w-5 h-5" />
        </button>

        <button
          @click="handleReset"
          class="p-2 rounded-full bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white cursor-pointer pointer-events-auto transition-colors"
          title="重置"
        >
          <Reset class="w-5 h-5" />
        </button>

        <button
          v-if="showCompleteAction"
          @click="handleCompleteAction"
          class="p-2 rounded-full bg-emerald-600/20 hover:bg-emerald-600 text-emerald-500 hover:text-white cursor-pointer pointer-events-auto transition-colors"
          title="完成任务"
        >
          <Checkmark class="w-5 h-5" />
        </button>

        <button
          v-if="showBreakAction"
          @click="handleBreakAction"
          class="p-2 rounded-full hover:bg-indigo-600 text-indigo-400 hover:text-white cursor-pointer pointer-events-auto transition-colors"
          title="休息"
        >
          <Cafe class="w-5 h-5" />
        </button>

        <button
          v-if="showModeToggleAction"
          @click="handleModeToggle"
          class="p-2 rounded-full hover:bg-slate-700 text-slate-300 hover:text-white cursor-pointer pointer-events-auto transition-colors"
          :title="modeToggleTitle"
        >
          <component :is="snapshot.mode === 'timer' ? ChartLineData : Timer" class="w-5 h-5" />
        </button>

        <button
          @click="expandToMain"
          class="p-2 rounded-full hover:bg-slate-700 text-slate-400 hover:text-white cursor-pointer pointer-events-auto transition-colors"
          title="返回主窗口"
        >
          <Maximize class="w-5 h-5" />
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.font-mono {
  font-variant-numeric: tabular-nums;
}
</style>
