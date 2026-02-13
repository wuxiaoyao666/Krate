<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import {
  Activity,
  Add,
  ArrowLeft,
  Cafe,
  ChartLineData,
  Checkmark,
  Close,
  Edit,
  Flash,
  List,
  Maximize,
  Minimize,
  Pause,
  Play,
  Reset,
  Timer,
  TrashCan,
} from '@vicons/carbon'
import type { FocusMode, FocusTask, TaskDraft } from '@/hooks/pomodoroTypes'
import {
  backToDashboard,
  completeCurrentTask,
  createTask,
  currentTask,
  currentView,
  deleteTask,
  displayTime,
  enterMiniWindowMode,
  endBreak,
  isOvertime,
  isRunning,
  mode,
  progressPercentage,
  resetTimer,
  startBreak,
  startFocus,
  statusText,
  tasks,
  themeClass,
  toggleMode,
  toggleTimer,
  updateTask,
} from '@/hooks/usePomodoro'

const showTaskModal = ref(false)
const editingTask = ref<FocusTask | null>(null)

const form = ref({
  title: '',
  mode: 'timer' as FocusMode,
  duration: 25,
  breakDuration: 5,
  tagInput: '',
})

const activeTaskId = computed(() => currentTask.value?.id ?? null)
const clampedProgress = computed(() => Math.max(0, Math.min(100, progressPercentage.value)))

const radius = 150
const stroke = 5
const normalizedRadius = radius - stroke * 2
const circumference = normalizedRadius * 2 * Math.PI

const strokeDashoffset = computed(() => {
  if (isOvertime.value) return 0
  return circumference - (clampedProgress.value / 100) * circumference
})

const strokeColorClass = computed(() => {
  if (isOvertime.value) return 'stroke-rose-500'
  if (mode.value === 'break') return 'stroke-indigo-400'
  return mode.value === 'timer' ? 'stroke-emerald-400' : 'stroke-sky-400'
})

function openCreate() {
  editingTask.value = null
  form.value = {
    title: '',
    mode: 'timer',
    duration: 25,
    breakDuration: 5,
    tagInput: '',
  }
  showTaskModal.value = true
}

function openEdit(task: FocusTask) {
  editingTask.value = task
  form.value = {
    title: task.title,
    mode: task.mode,
    duration: task.duration,
    breakDuration: task.breakDuration,
    tagInput: task.tags.join(' '),
  }
  showTaskModal.value = true
}

function closeTaskModal() {
  showTaskModal.value = false
  window.setTimeout(() => {
    editingTask.value = null
  }, 200)
}

function submitTask() {
  if (!form.value.title.trim()) return

  const payload: TaskDraft = {
    title: form.value.title,
    mode: form.value.mode,
    duration: form.value.duration,
    breakDuration: form.value.breakDuration,
    tags: form.value.tagInput.split(/[,， ]+/).filter((tag) => tag.trim()),
  }

  if (editingTask.value) {
    updateTask(editingTask.value.id, payload)
  } else {
    createTask(payload)
  }

  closeTaskModal()
}

function handleMainAction() {
  if (mode.value === 'break') {
    endBreak()
    return
  }
  toggleTimer()
}

function handleTakeBreak() {
  startBreak(currentTask.value?.breakDuration || 5)
}

function startFreeTimer() {
  startFocus(undefined, { autoMini: false })
  if (mode.value === 'stopwatch') {
    toggleMode()
  }
}

function goDashboard() {
  backToDashboard()
}

function openMini() {
  void enterMiniWindowMode()
}

function isEditableTarget(target: EventTarget | null) {
  const element = target as HTMLElement | null
  if (!element) return false
  if (element.isContentEditable) return true
  const tag = element.tagName
  return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT'
}

function handleMiniShortcut(event: KeyboardEvent) {
  const hitMiniShortcut = (event.metaKey || event.ctrlKey) && event.shiftKey && event.key.toLowerCase() === 'm'
  if (!hitMiniShortcut) return
  if (isEditableTarget(event.target)) return

  const hasFocusContext = currentView.value === 'focus' || activeTaskId.value !== null || isRunning.value
  if (!hasFocusContext) return

  event.preventDefault()
  void enterMiniWindowMode()
}

function handleBottomPrimaryAction() {
  if (activeTaskId.value) {
    startFocus(activeTaskId.value, { autoMini: false })
    return
  }

  startFreeTimer()
}

function handleTaskPrimaryAction(taskId: number) {
  if (taskId === activeTaskId.value) {
    startFocus(taskId, { autoMini: false })
    return
  }
  startFocus(taskId)
}

onMounted(() => {
  window.addEventListener('keydown', handleMiniShortcut)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleMiniShortcut)
})
</script>

<template>
  <div
    @contextmenu.prevent
    class="h-full w-full overflow-hidden text-slate-200 select-none flex flex-col relative font-sans"
  >
    <div class="flex-1 flex flex-col relative bg-slate-950 rounded-4xl border border-slate-800 overflow-hidden">
      <transition name="fade" mode="out-in">
        <div
          v-if="currentView === 'dashboard'"
          key="dashboard"
          class="flex-1 flex flex-col h-full relative pt-8 dashboard-stage"
        >
          <div class="absolute top-5 right-6 z-30">
            <button
              @click="openMini"
              class="p-2 text-slate-500 hover:text-white transition rounded-lg hover:bg-white/5 cursor-pointer bg-slate-950/50 backdrop-blur-sm border border-white/5"
              title="迷你模式"
            >
              <Minimize class="w-4.5 h-4.5" />
            </button>
          </div>

          <div class="px-6 pt-4 pb-4">
            <h1 class="text-2xl font-bold text-slate-100 flex items-center gap-2">
              <List class="w-6 h-6 text-emerald-400" />
              今日任务
            </h1>
            <p class="text-slate-500 text-sm mt-1">保持专注，逐个击破。</p>
          </div>

          <div class="flex-1 overflow-y-auto px-6 pb-24 scrollbar-hide relative">
            <transition-group name="task" tag="div" class="space-y-3">
              <div
                v-for="(task, index) in tasks"
                :key="task.id"
                class="group rounded-xl p-4 transition-all cursor-default flex items-center justify-between border relative overflow-hidden"
                :style="{ transitionDelay: `${Math.min(index * 35, 180)}ms` }"
                :class="[
                  task.isCompleted
                    ? 'bg-slate-900/20 border-slate-800/50 opacity-60 hover:opacity-100'
                    : 'bg-slate-900/50 border-slate-800 hover:border-slate-700 hover:bg-slate-900',

                  task.id === activeTaskId
                    ? 'bg-emerald-900/20 border-emerald-500 shadow-[0_0_20px_rgba(16,185,129,0.15)] z-10'
                    : '',
                ]"
              >
                <div
                  v-if="task.id === activeTaskId"
                  class="absolute inset-0 bg-emerald-500/5 animate-pulse pointer-events-none"
                ></div>

                <div class="flex items-center gap-4 min-w-0 z-10">
                  <div
                    class="w-10 h-10 rounded-full flex items-center justify-center shrink-0 transition-colors"
                    :class="[
                      task.isCompleted
                        ? 'bg-slate-800 text-slate-600'
                        : task.id === activeTaskId
                          ? 'bg-emerald-500 text-white'
                          : task.mode === 'timer'
                            ? 'bg-emerald-500/10 text-emerald-500'
                            : 'bg-sky-500/10 text-sky-500',
                    ]"
                  >
                    <Activity v-if="task.id === activeTaskId" class="w-5 h-5 animate-pulse" />
                    <component :is="task.mode === 'timer' ? Timer : ChartLineData" v-else class="w-5 h-5" />
                  </div>

                  <div class="flex flex-col min-w-0">
                    <div class="flex items-center gap-2">
                      <span
                        class="text-base font-medium truncate transition-all"
                        :class="[
                          task.isCompleted ? 'line-through text-slate-500' : '',
                          task.id === activeTaskId ? 'text-emerald-400' : 'text-slate-200',
                        ]"
                      >
                        {{ task.title }}
                      </span>

                      <span
                        v-if="task.id === activeTaskId"
                        class="text-[10px] bg-emerald-500/20 text-emerald-400 px-1.5 py-0.5 rounded border border-emerald-500/30 animate-pulse"
                      >
                        进行中
                      </span>
                    </div>

                    <div class="flex items-center gap-3 text-xs text-slate-500 mt-0.5">
                      <span class="flex items-center gap-1" v-if="task.mode === 'timer'">
                        <Timer class="w-3 h-3" /> {{ task.duration }} 分钟
                      </span>
                      <span class="flex items-center gap-1" v-else>
                        <ChartLineData class="w-3 h-3" /> 正向计时
                      </span>
                      <span class="flex gap-1" v-if="task.tags && task.tags.length">
                        <span
                          v-for="tag in task.tags"
                          :key="`${task.id}-${tag}`"
                          class="bg-slate-800 px-1.5 rounded text-slate-400"
                        >
                          #{{ tag }}
                        </span>
                      </span>
                    </div>
                  </div>
                </div>

                <div class="flex items-center gap-1 z-10">
                  <button
                    @click="handleTaskPrimaryAction(task.id)"
                    class="p-2 rounded-lg transition-colors cursor-pointer"
                    :class="
                      task.id === activeTaskId
                        ? 'bg-emerald-500 hover:bg-emerald-400 text-white shadow-lg shadow-emerald-900/50'
                        : 'bg-slate-800 hover:bg-emerald-600 text-emerald-500 hover:text-white'
                    "
                    :title="task.id === activeTaskId ? '回到专注页面' : '开始此任务'"
                  >
                    <component :is="task.id === activeTaskId ? Maximize : Play" class="w-4.5 h-4.5" />
                  </button>

                  <button
                    v-if="task.id !== activeTaskId"
                    @click="openEdit(task)"
                    class="p-2 text-slate-600 hover:text-sky-400 hover:bg-sky-950/30 rounded-lg transition-all opacity-0 group-hover:opacity-100 cursor-pointer"
                    title="编辑任务"
                  >
                    <Edit class="w-4.5 h-4.5" />
                  </button>

                  <button
                    v-if="task.id !== activeTaskId"
                    @click="deleteTask(task.id)"
                    class="p-2 text-slate-600 hover:text-rose-400 hover:bg-rose-950/30 rounded-lg transition-all opacity-0 group-hover:opacity-100 cursor-pointer"
                    title="删除任务"
                  >
                    <TrashCan class="w-4.5 h-4.5" />
                  </button>
                </div>
              </div>
            </transition-group>

            <div
              v-if="tasks.length === 0"
              class="flex flex-col items-center justify-center py-20 text-slate-600 h-full"
            >
              <div class="w-16 h-16 bg-slate-900 rounded-full flex items-center justify-center mb-4">
                <List class="w-8 h-8 opacity-50" />
              </div>
              <p>还没有任务，添加一个开始吧</p>
            </div>
          </div>

          <div
            class="h-20 border-t border-slate-800 bg-slate-950/80 backdrop-blur-md absolute bottom-0 left-0 right-0 px-6 flex items-center justify-between z-20"
          >
            <div class="flex items-center gap-2">
              <button
                @click="handleBottomPrimaryAction"
                class="flex items-center gap-2 text-slate-400 hover:text-white transition font-medium px-4 py-2 hover:bg-slate-900 rounded-lg cursor-pointer"
                :class="activeTaskId ? 'text-emerald-400 animate-pulse' : ''"
              >
                <component :is="activeTaskId ? Activity : Flash" class="w-4.5 w-4.5" />
                {{ activeTaskId ? '回到专注' : '自由专注' }}
              </button>
            </div>

            <button
              @click="openCreate"
              class="bg-emerald-600 hover:bg-emerald-500 text-white shadow-lg shadow-emerald-900/40 px-6 py-3 rounded-xl flex items-center gap-2 font-bold transition active:scale-95 cursor-pointer"
            >
              <Add class="w-5 h-5" />
              新建任务
            </button>
          </div>
        </div>

        <div
          v-else
          key="focus"
          class="flex-1 flex flex-col h-full relative overflow-hidden bg-slate-950 focus-stage"
        >
          <div
            class="absolute top-0 left-0 right-0 h-14 flex items-center justify-between px-6 mt-6 z-50 pointer-events-none"
          >
            <button
              @click="goDashboard"
              class="pointer-events-auto p-2 -ml-2 text-slate-500 hover:text-white transition rounded-lg hover:bg-white/5 flex items-center gap-2 cursor-pointer bg-slate-950/50 backdrop-blur-sm border border-white/5"
              title="返回清单"
            >
              <ArrowLeft class="w-[18px] w-4.5" />
              <span class="text-sm font-medium">列表</span>
            </button>

            <button
              @click="openMini"
              class="pointer-events-auto p-2 text-slate-500 hover:text-white transition rounded-lg hover:bg-white/5 cursor-pointer bg-slate-950/50 backdrop-blur-sm border border-white/5"
              title="迷你模式"
            >
              <Minimize class="w-[18px] w-4.5" />
            </button>
          </div>

          <div class="flex-1 flex flex-col items-center justify-center relative z-10 w-full">
            <template v-if="!isOvertime">
              <div class="mb-6 flex flex-col items-center gap-2">
                <div
                  v-if="mode === 'break'"
                  class="flex items-center gap-2 px-4 py-1.5 rounded-full bg-indigo-500/10 border border-indigo-500/20 text-sm text-indigo-300 shadow-xl"
                >
                  <Cafe class="w-[14px] h-[14px] text-indigo-400" />
                  <span class="font-medium">休息模式</span>
                </div>
                <div
                  v-else-if="currentTask"
                  class="flex items-center gap-2 px-4 py-1.5 rounded-full bg-slate-900 border border-slate-800 text-sm text-slate-300 shadow-xl"
                >
                  <component
                    :is="currentTask.mode === 'timer' ? Timer : ChartLineData"
                    class="w-[14px] h-[14px]"
                    :class="themeClass"
                  />
                  <span class="font-medium max-w-[200px] truncate">{{ currentTask.title }}</span>
                </div>
                <div v-else class="text-slate-500 text-sm font-medium tracking-wide uppercase">自由专注模式</div>
              </div>

              <div class="relative flex items-center justify-center mb-8">
                <svg
                  :height="radius * 2"
                  :width="radius * 2"
                  class="transform -rotate-90 pointer-events-none"
                >
                  <circle
                    class="text-slate-800/50 stroke-current"
                    :stroke-width="stroke"
                    fill="transparent"
                    :r="normalizedRadius"
                    :cx="radius"
                    :cy="radius"
                  />
                  <circle
                    class="transition-all duration-1000 ease-linear"
                    :class="[strokeColorClass, isRunning ? 'filter drop-shadow-[0_0_8px_rgba(52,211,153,0.4)]' : '']"
                    :stroke-width="stroke"
                    :stroke-dasharray="circumference + ' ' + circumference"
                    :style="{ strokeDashoffset }"
                    stroke-linecap="round"
                    fill="transparent"
                    :r="normalizedRadius"
                    :cx="radius"
                    :cy="radius"
                  />
                </svg>

                <div class="absolute inset-0 flex items-center justify-center z-10">
                  <div
                    class="absolute inset-0 rounded-full blur-[60px] opacity-10 transition-all duration-1000"
                    :class="[
                      mode === 'break' ? 'bg-indigo-500' : mode === 'timer' ? 'bg-emerald-500' : 'bg-sky-500',
                      isRunning ? 'opacity-20 scale-110' : 'opacity-5 scale-100',
                    ]"
                  ></div>

                  <h1
                    class="relative font-mono text-7xl font-bold tracking-wider drop-shadow-2xl transition-colors duration-300 select-none"
                    :class="themeClass"
                  >
                    {{ displayTime }}
                  </h1>
                </div>
              </div>

              <transition name="status" mode="out-in">
                <div
                  :key="`${mode}-${isRunning}-${isOvertime}`"
                  class="h-6 mb-6 text-slate-500 text-sm transition-all duration-300 flex items-center gap-2"
                >
                  <component :is="mode === 'timer' ? Timer : mode === 'break' ? Cafe : ChartLineData" class="w-4 h-4" />
                  <span>{{ statusText }}</span>
                </div>
              </transition>

              <div class="flex items-center gap-6">
                <button
                  @click="resetTimer"
                  class="p-3 rounded-full text-slate-500 hover:text-slate-200 hover:bg-slate-800 transition active:scale-95 cursor-pointer"
                  title="放弃/重置"
                >
                  <Reset class="w-5 h-5" />
                </button>

                <button
                  v-if="mode !== 'break'"
                  @click="handleTakeBreak"
                  class="p-3 rounded-full text-slate-500 hover:text-indigo-400 hover:bg-indigo-500/10 transition active:scale-95 cursor-pointer"
                  title="休息一下"
                >
                  <Cafe class="w-5 h-5" />
                </button>
                <div v-else class="w-[44px]"></div>

                <button
                  @click="handleMainAction"
                  class="p-6 rounded-[2rem] transition-all duration-300 shadow-2xl hover:shadow-emerald-500/20 active:scale-95 flex items-center justify-center border border-white/5 cursor-pointer bg-slate-900 group"
                  :class="isRunning ? 'ring-2 ring-emerald-500/20' : 'hover:bg-slate-800'"
                  :title="mode === 'break' ? '结束休息' : isRunning ? '暂停' : '开始'"
                >
                  <component
                    :is="mode === 'break' ? Play : isRunning ? Pause : Play"
                    class="w-9 h-9 transition-colors duration-300"
                    :class="mode === 'break' ? 'text-indigo-400' : mode === 'timer' ? 'text-emerald-500' : 'text-sky-500'"
                  />
                </button>

                <button
                  v-if="currentTask"
                  @click="completeCurrentTask"
                  class="p-3 rounded-full text-slate-500 hover:text-emerald-500 hover:bg-emerald-500/10 transition active:scale-95 cursor-pointer"
                  title="提前完成任务"
                >
                  <Checkmark class="w-5 h-5" />
                </button>

                <button
                  v-else
                  @click="toggleMode"
                  class="p-3 rounded-full text-slate-500 hover:text-slate-200 hover:bg-slate-800 transition active:scale-95 cursor-pointer"
                  :title="mode === 'timer' ? '切换到正计时' : '切换到倒计时'"
                >
                  <component :is="mode === 'timer' ? ChartLineData : Timer" class="w-5 h-5" />
                </button>
              </div>
            </template>

            <template v-else>
              <div class="mb-8">
                <div
                  class="flex items-center gap-2 px-4 py-1.5 rounded-full bg-rose-500/10 border border-rose-500/20 text-sm text-rose-400 shadow-xl"
                >
                  <span class="relative flex h-2 w-2">
                    <span
                      class="animate-ping absolute inline-flex h-full w-full rounded-full bg-rose-400 opacity-75"
                    ></span>
                    <span class="relative inline-flex rounded-full h-2 w-2 bg-rose-500"></span>
                  </span>
                  <span class="font-bold tracking-wide">超时模式</span>
                </div>
              </div>

              <div class="relative flex items-center justify-center mb-10">
                <svg
                  :height="radius * 2"
                  :width="radius * 2"
                  class="transform -rotate-90 pointer-events-none"
                >
                  <circle
                    class="text-rose-500 stroke-current animate-pulse-slow filter drop-shadow-[0_0_15px_rgba(244,63,94,0.5)]"
                    :stroke-width="stroke"
                    fill="transparent"
                    :r="normalizedRadius"
                    :cx="radius"
                    :cy="radius"
                  />
                </svg>

                <div class="absolute inset-0 flex flex-col items-center justify-center z-10">
                  <h1 class="relative font-mono text-7xl font-bold tracking-wider drop-shadow-2xl select-none text-rose-500 animate-pulse">
                    {{ displayTime }}
                  </h1>
                  <div class="absolute bottom-16 text-rose-500/50 font-medium tracking-widest text-sm uppercase">Overtime</div>
                </div>
              </div>

              <div class="flex items-center gap-5">
                <button
                  @click="handleTakeBreak"
                  class="flex flex-col items-center gap-2 p-5 rounded-2xl bg-slate-900 border border-slate-800 hover:border-indigo-500/50 hover:bg-slate-800 transition-all duration-300 min-w-[120px] cursor-pointer group"
                >
                  <div
                    class="p-3 rounded-full transition-all duration-300 group-hover:scale-110 bg-indigo-500/10 text-indigo-400 group-hover:bg-indigo-500 group-hover:text-white"
                  >
                    <Cafe class="w-6 h-6" />
                  </div>
                  <span class="text-indigo-200 font-medium text-sm">休息一下</span>
                </button>

                <button
                  @click="completeCurrentTask"
                  class="flex flex-col items-center gap-2 p-5 rounded-2xl bg-slate-900 border border-slate-800 hover:border-emerald-500/50 hover:bg-slate-800 transition-all duration-300 min-w-[120px] cursor-pointer group"
                >
                  <div
                    class="p-3 rounded-full transition-all duration-300 group-hover:scale-110 bg-emerald-500/10 text-emerald-400 group-hover:bg-emerald-500 group-hover:text-white"
                  >
                    <Checkmark class="w-6 h-6" />
                  </div>
                  <span class="text-emerald-200 font-medium text-sm">完成任务</span>
                </button>
              </div>
            </template>
          </div>
        </div>
      </transition>

      <transition name="fade">
        <div
          v-if="showTaskModal"
          class="absolute inset-0 bg-slate-950/80 backdrop-blur-sm z-100 flex items-end sm:items-center justify-center p-4"
        >
          <div class="w-full max-w-sm bg-slate-900 border border-slate-700 rounded-xl shadow-2xl p-5 space-y-4">
            <div class="flex justify-between items-center">
              <h3 class="text-slate-200 font-semibold">{{ editingTask ? '编辑任务' : '新建任务' }}</h3>
              <button @click="closeTaskModal" class="text-slate-500 hover:text-white cursor-pointer">
                <Close class="w-4.5" />
              </button>
            </div>

            <div class="space-y-3">
              <div>
                <label class="text-xs text-slate-500 block mb-1">任务名称</label>
                <input
                  v-model="form.title"
                  type="text"
                  placeholder="例如：阅读《xxx》"
                  class="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-sm text-white focus:border-emerald-500 focus:outline-none transition"
                  autofocus
                  @keydown.enter="submitTask"
                />
              </div>

              <div>
                <label class="text-xs text-slate-500 block mb-1">模式选择</label>
                <div class="grid grid-cols-2 gap-2">
                  <button
                    @click="form.mode = 'timer'"
                    class="flex items-center justify-center gap-2 py-2 rounded-lg border text-sm transition cursor-pointer"
                    :class="
                      form.mode === 'timer'
                        ? 'bg-emerald-600 border-emerald-600 text-white'
                        : 'bg-slate-950 border-slate-800 text-slate-400 hover:border-slate-600'
                    "
                  >
                    <Timer class="w-[14px] h-[14px]" /> 专注计时
                  </button>
                  <button
                    @click="form.mode = 'stopwatch'"
                    class="flex items-center justify-center gap-2 py-2 rounded-lg border text-sm transition cursor-pointer"
                    :class="
                      form.mode === 'stopwatch'
                        ? 'bg-sky-600 border-sky-600 text-white'
                        : 'bg-slate-950 border-slate-800 text-slate-400 hover:border-slate-600'
                    "
                  >
                    <ChartLineData class="w-[14px] h-[14px]" /> 正向计时
                  </button>
                </div>
              </div>

              <div v-if="form.mode === 'timer'">
                <label class="text-xs text-slate-500 block mb-1">专注时长 (分钟)</label>
                <input
                  v-model.number="form.duration"
                  type="number"
                  min="1"
                  max="180"
                  class="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-sm text-white focus:border-emerald-500 focus:outline-none transition"
                  @keydown.enter="submitTask"
                />
              </div>

              <div>
                <label class="text-xs text-slate-500 block mb-1">休息时长 (分钟)</label>
                <input
                  v-model.number="form.breakDuration"
                  type="number"
                  min="1"
                  max="60"
                  class="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-sm text-white focus:border-indigo-500 focus:outline-none transition"
                  @keydown.enter="submitTask"
                />
              </div>

              <div>
                <label class="text-xs text-slate-500 block mb-1">标签 (空格分隔)</label>
                <input
                  v-model="form.tagInput"
                  type="text"
                  placeholder="阅读 学习"
                  class="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-sm text-white focus:border-emerald-500 focus:outline-none transition"
                  @keydown.enter="submitTask"
                />
              </div>
            </div>

            <div class="grid grid-cols-3 gap-3 pt-2">
              <button
                @click="closeTaskModal"
                class="col-span-1 bg-slate-800 hover:bg-slate-700 text-slate-300 font-medium py-2.5 rounded-lg transition active:scale-95 cursor-pointer"
              >
                取消
              </button>
              <button
                @click="submitTask"
                class="col-span-2 bg-slate-100 hover:bg-white text-slate-900 font-bold py-2.5 rounded-lg transition active:scale-95 cursor-pointer"
              >
                {{ editingTask ? '保存修改' : '立即创建' }}
              </button>
            </div>
          </div>
        </div>
      </transition>
    </div>

  </div>
</template>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

.status-enter-active,
.status-leave-active {
  transition:
    opacity 0.2s ease,
    transform 0.2s ease;
}

.status-enter-from,
.status-leave-to {
  opacity: 0;
  transform: translateY(8px);
}

.task-enter-active,
.task-leave-active {
  transition:
    opacity 0.26s ease,
    transform 0.26s ease;
}

.task-enter-from,
.task-leave-to {
  opacity: 0;
  transform: translateY(14px) scale(0.98);
}

.task-move {
  transition: transform 0.26s ease;
}

.scrollbar-hide::-webkit-scrollbar {
  display: none;
}

.font-mono {
  font-variant-numeric: tabular-nums;
}

.animate-pulse-slow {
  animation: pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}

.dashboard-stage {
  animation: stageIn 0.28s ease-out;
}

.focus-stage {
  animation: stageIn 0.3s ease-out;
}

@keyframes stageIn {
  from {
    opacity: 0;
    transform: translateY(10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.7;
  }
}

input[type='number']::-webkit-inner-spin-button,
input[type='number']::-webkit-outer-spin-button {
  -webkit-appearance: none;
  margin: 0;
}
</style>
