import { computed, ref, watchEffect } from 'vue'
import type { FocusMode, FocusTask, RunningMode, TaskDraft } from './pomodoroTypes'
import {
  closePomodoroMiniWindow,
  emitPomodoroRuntimeSnapshot,
  enterPomodoroMiniWindowMode,
  listenPomodoroMiniAction,
  restoreMainWindow,
  type PomodoroMiniAction,
  type PomodoroViewState,
} from './usePomodoroMiniBridge'

const TASK_STORAGE_KEY = 'krate.time.pomodoro.tasks.v1'
const LEGACY_TASK_STORAGE_KEYS = ['krate.aura.tasks.v1']

const DEFAULT_TIMER_SECONDS = 25 * 60
const DEFAULT_BREAK_MINUTES = 5
const DEFAULT_STOPWATCH_TARGET_SECONDS = 30 * 60

const currentView = ref<PomodoroViewState>('dashboard')
const isMini = ref(false)

const mode = ref<RunningMode>('timer')
const isRunning = ref(false)
const timeLeft = ref(DEFAULT_TIMER_SECONDS)
const timeElapsed = ref(0)
const timerDuration = ref(DEFAULT_TIMER_SECONDS)
const isOvertime = ref(false)
const overtimeSeconds = ref(0)

const tasks = ref<FocusTask[]>(loadTasksFromStorage())
const currentTaskId = ref<number | null>(null)

let timerInterval: number | null = null
let countdownEndAt = 0
let elapsedStartAt = 0
let miniActionUnlisten: (() => void) | null = null

const currentTask = computed(() => {
  return tasks.value.find((task) => task.id === currentTaskId.value) ?? null
})

const activeTasks = computed(() => tasks.value.filter((task) => !task.isCompleted))
const completedTasks = computed(() => tasks.value.filter((task) => task.isCompleted))

export const formatTime = (seconds: number) => {
  const safeSeconds = Math.max(0, Math.floor(seconds))
  const m = Math.floor(safeSeconds / 60)
    .toString()
    .padStart(2, '0')
  const s = (safeSeconds % 60).toString().padStart(2, '0')
  return `${m}:${s}`
}

const displayTime = computed(() => {
  if (isOvertime.value) {
    return `+ ${formatTime(overtimeSeconds.value)}`
  }

  if (mode.value === 'stopwatch') {
    return formatTime(timeElapsed.value)
  }

  return formatTime(timeLeft.value)
})

const progressPercentage = computed(() => {
  if (isOvertime.value) return 100

  if (mode.value === 'timer' || mode.value === 'break') {
    if (timerDuration.value <= 0) return 0
    return (timeLeft.value / timerDuration.value) * 100
  }

  const target = currentTask.value?.duration
    ? currentTask.value.duration * 60
    : DEFAULT_STOPWATCH_TARGET_SECONDS
  return Math.min((timeElapsed.value / target) * 100, 100)
})

const statusText = computed(() => {
  if (mode.value === 'break') return isRunning.value ? '休息中...' : '休息已暂停'
  if (isOvertime.value) return '已超时，建议休息或结束当前任务'

  if (mode.value === 'stopwatch') {
    return isRunning.value ? '正计时进行中' : '正计时已暂停'
  }

  if (isRunning.value) return '保持专注'
  if (timeLeft.value === timerDuration.value) return '准备开始'
  return '计时已暂停'
})

const themeClass = computed(() => {
  if (isOvertime.value) return 'text-rose-400'
  if (mode.value === 'break') return 'text-indigo-300'
  return mode.value === 'timer' ? 'text-emerald-300' : 'text-cyan-300'
})

const progressBarClass = computed(() => {
  if (isOvertime.value) return 'bg-rose-500'
  if (mode.value === 'break') return 'bg-indigo-500'
  return mode.value === 'timer' ? 'bg-emerald-500' : 'bg-cyan-500'
})

function loadTasksFromStorage(): FocusTask[] {
  if (typeof localStorage === 'undefined') return []
  const candidateKeys = [TASK_STORAGE_KEY, ...LEGACY_TASK_STORAGE_KEYS]

  let raw: string | null = null
  let loadedKey: string | null = null

  for (const key of candidateKeys) {
    const value = localStorage.getItem(key)
    if (value) {
      raw = value
      loadedKey = key
      break
    }
  }

  if (!raw) return []

  try {
    const parsed = JSON.parse(raw) as unknown
    if (!Array.isArray(parsed)) return []

    const normalized = parsed.map((item, index) => normalizeTask(item, index))
    const sorted = sortTasks(normalized)

    if (loadedKey && loadedKey !== TASK_STORAGE_KEY) {
      localStorage.setItem(TASK_STORAGE_KEY, JSON.stringify(sorted))
    }

    return sorted
  } catch (error) {
    console.error('读取本地任务失败:', error)
    return []
  }
}

function saveTasksToStorage() {
  if (typeof localStorage === 'undefined') return
  localStorage.setItem(TASK_STORAGE_KEY, JSON.stringify(tasks.value))
}

function sortTasks(input: FocusTask[]) {
  return [...input].sort((a, b) => {
    const byComplete = Number(a.isCompleted) - Number(b.isCompleted)
    if (byComplete !== 0) return byComplete
    return Date.parse(b.createdAt) - Date.parse(a.createdAt)
  })
}

function normalizeTask(raw: unknown, index: number): FocusTask {
  const source = typeof raw === 'object' && raw !== null ? (raw as Record<string, unknown>) : {}
  const fallbackId = Date.now() + index

  const title = typeof source.title === 'string' ? source.title.trim() : ''
  const taskMode = source.mode === 'stopwatch' ? 'stopwatch' : 'timer'
  const duration = toPositiveInt(source.duration, 25)
  const breakDuration = toPositiveInt(source.breakDuration, DEFAULT_BREAK_MINUTES)

  const tags = Array.isArray(source.tags)
    ? source.tags
        .filter((tag): tag is string => typeof tag === 'string')
        .map((tag) => tag.trim())
        .filter(Boolean)
    : []

  const createdAt =
    typeof source.createdAt === 'string' && !Number.isNaN(Date.parse(source.createdAt))
      ? source.createdAt
      : new Date().toISOString()

  const updatedAt =
    typeof source.updatedAt === 'string' && !Number.isNaN(Date.parse(source.updatedAt))
      ? source.updatedAt
      : createdAt

  return {
    id: toPositiveInt(source.id, fallbackId),
    title: title || `未命名任务 ${index + 1}`,
    mode: taskMode,
    duration,
    breakDuration,
    tags,
    isCompleted: source.isCompleted === true,
    createdAt,
    updatedAt,
  }
}

function toPositiveInt(value: unknown, fallback: number) {
  const num = typeof value === 'number' ? value : Number(value)
  if (!Number.isFinite(num) || num <= 0) return fallback
  return Math.floor(num)
}

function nextTaskId() {
  const maxId = tasks.value.reduce((max, task) => Math.max(max, task.id), 0)
  return maxId + 1
}

function stopTicker() {
  if (timerInterval !== null) {
    clearInterval(timerInterval)
    timerInterval = null
  }
  isRunning.value = false
}

function startTicker() {
  if (isRunning.value) return
  isRunning.value = true

  const now = Date.now()

  if (mode.value === 'timer' || mode.value === 'break') {
    if (isOvertime.value) {
      elapsedStartAt = now - overtimeSeconds.value * 1000
    } else {
      countdownEndAt = now + timeLeft.value * 1000
    }
  } else {
    elapsedStartAt = now - timeElapsed.value * 1000
  }

  timerInterval = window.setInterval(() => {
    const currentNow = Date.now()

    if (mode.value === 'timer' || mode.value === 'break') {
      if (isOvertime.value) {
        overtimeSeconds.value = Math.floor((currentNow - elapsedStartAt) / 1000)
        return
      }

      const remaining = Math.max(0, Math.ceil((countdownEndAt - currentNow) / 1000))
      timeLeft.value = remaining

      if (remaining > 0) return

      if (mode.value === 'break') {
        stopTicker()
        isOvertime.value = false
        overtimeSeconds.value = 0

        if (currentTask.value) {
          mode.value = currentTask.value.mode
          if (currentTask.value.mode === 'timer') {
            timerDuration.value = currentTask.value.duration * 60
            timeLeft.value = timerDuration.value
          } else {
            timeElapsed.value = 0
          }
        } else {
          mode.value = 'timer'
          timerDuration.value = DEFAULT_TIMER_SECONDS
          timeLeft.value = DEFAULT_TIMER_SECONDS
          timeElapsed.value = 0
        }
      } else {
        isOvertime.value = true
        overtimeSeconds.value = 0
        elapsedStartAt = currentNow
      }
      return
    }

    timeElapsed.value = Math.floor((currentNow - elapsedStartAt) / 1000)
  }, 200)
}

function pauseTimer() {
  stopTicker()
}

function toggleTimer() {
  if (isRunning.value) {
    pauseTimer()
    return
  }
  startTicker()
}

function switchMode(nextMode: FocusMode) {
  mode.value = nextMode
  resetTimer()
}

function toggleMode() {
  if (mode.value === 'break') return
  switchMode(mode.value === 'timer' ? 'stopwatch' : 'timer')
}

function resetTimer() {
  pauseTimer()
  isOvertime.value = false
  overtimeSeconds.value = 0

  if (mode.value === 'break') {
    timerDuration.value = DEFAULT_BREAK_MINUTES * 60
    timeLeft.value = timerDuration.value
    return
  }

  if (currentTask.value) {
    if (currentTask.value.mode === 'timer') {
      mode.value = 'timer'
      timerDuration.value = currentTask.value.duration * 60
      timeLeft.value = timerDuration.value
      timeElapsed.value = 0
    } else {
      mode.value = 'stopwatch'
      timeElapsed.value = 0
      timeLeft.value = DEFAULT_TIMER_SECONDS
      timerDuration.value = DEFAULT_TIMER_SECONDS
    }
    return
  }

  if (mode.value === 'stopwatch') {
    timeElapsed.value = 0
    timerDuration.value = DEFAULT_TIMER_SECONDS
    timeLeft.value = DEFAULT_TIMER_SECONDS
    return
  }

  mode.value = 'timer'
  timerDuration.value = DEFAULT_TIMER_SECONDS
  timeLeft.value = DEFAULT_TIMER_SECONDS
  timeElapsed.value = 0
}

function startBreak(minutes = DEFAULT_BREAK_MINUTES) {
  pauseTimer()
  isOvertime.value = false
  overtimeSeconds.value = 0

  mode.value = 'break'
  timerDuration.value = toPositiveInt(minutes, DEFAULT_BREAK_MINUTES) * 60
  timeLeft.value = timerDuration.value

  startTicker()
}

function endBreak() {
  pauseTimer()
  isOvertime.value = false
  overtimeSeconds.value = 0

  if (currentTask.value) {
    mode.value = currentTask.value.mode
    if (currentTask.value.mode === 'timer') {
      timerDuration.value = currentTask.value.duration * 60
      timeLeft.value = timerDuration.value
      timeElapsed.value = 0
    } else {
      timeElapsed.value = 0
    }
  } else {
    mode.value = 'timer'
    timerDuration.value = DEFAULT_TIMER_SECONDS
    timeLeft.value = DEFAULT_TIMER_SECONDS
    timeElapsed.value = 0
  }

  startTicker()
  currentView.value = 'focus'
}

async function enterMiniWindowMode() {
  isMini.value = true

  try {
    await enterPomodoroMiniWindowMode()
  } catch (error) {
    console.error('进入 mini 窗口失败:', error)
    isMini.value = false
  }
}

async function closeMiniWindowMode(options: { restoreMain?: boolean } = {}) {
  const { restoreMain = false } = options
  isMini.value = false

  try {
    await closePomodoroMiniWindow()
  } catch (error) {
    console.error('关闭 mini 窗口失败:', error)
  }

  if (!restoreMain) return

  try {
    await restoreMainWindow()
  } catch (error) {
    console.error('恢复主窗口失败:', error)
  }
}

function startFocus(taskId?: number, options: { autoMini?: boolean } = {}) {
  const task = typeof taskId === 'number' ? tasks.value.find((item) => item.id === taskId) : undefined
  if (task?.isCompleted) return

  const { autoMini = true } = options
  const isSameTask = task && task.id === currentTaskId.value
  const isSameFreeMode = !task && currentTaskId.value === null

  if (isSameTask || isSameFreeMode) {
    currentView.value = 'focus'
    return
  }

  isOvertime.value = false
  overtimeSeconds.value = 0
  pauseTimer()

  if (task) {
    currentTaskId.value = task.id
    mode.value = task.mode

    if (task.mode === 'timer') {
      timerDuration.value = task.duration * 60
      timeLeft.value = timerDuration.value
      timeElapsed.value = 0
    } else {
      timeElapsed.value = 0
    }

    startTicker()

    if (autoMini) {
      void enterMiniWindowMode()
      return
    }
  } else {
    currentTaskId.value = null
    const freeMode: FocusMode = mode.value === 'stopwatch' ? 'stopwatch' : 'timer'
    mode.value = freeMode
    timerDuration.value = DEFAULT_TIMER_SECONDS
    timeLeft.value = DEFAULT_TIMER_SECONDS
    timeElapsed.value = 0
  }

  currentView.value = 'focus'
}

function backToDashboard() {
  currentView.value = 'dashboard'
}

function createTask(draft: TaskDraft) {
  const title = draft.title.trim()
  if (!title) return

  const now = new Date().toISOString()

  const newTask: FocusTask = {
    id: nextTaskId(),
    title,
    mode: draft.mode,
    duration: toPositiveInt(draft.duration, 25),
    breakDuration: toPositiveInt(draft.breakDuration, DEFAULT_BREAK_MINUTES),
    tags: draft.tags.map((tag) => tag.trim()).filter(Boolean),
    isCompleted: false,
    createdAt: now,
    updatedAt: now,
  }

  tasks.value = sortTasks([newTask, ...tasks.value])
  saveTasksToStorage()
}

function updateTask(id: number, draft: TaskDraft) {
  const index = tasks.value.findIndex((task) => task.id === id)
  if (index === -1) return

  const target = tasks.value[index]
  const title = draft.title.trim()

  tasks.value[index] = {
    ...target,
    title: title || target.title,
    mode: draft.mode,
    duration: toPositiveInt(draft.duration, target.duration),
    breakDuration: toPositiveInt(draft.breakDuration, target.breakDuration),
    tags: draft.tags.map((tag) => tag.trim()).filter(Boolean),
    updatedAt: new Date().toISOString(),
  }

  tasks.value = sortTasks(tasks.value)
  saveTasksToStorage()

  if (currentTaskId.value === id && mode.value !== 'break') {
    resetTimer()
    if (currentView.value === 'focus') {
      startTicker()
    }
  }
}

function setTaskCompleted(id: number, completed: boolean) {
  const index = tasks.value.findIndex((task) => task.id === id)
  if (index === -1) return

  tasks.value[index] = {
    ...tasks.value[index],
    isCompleted: completed,
    updatedAt: new Date().toISOString(),
  }

  tasks.value = sortTasks(tasks.value)
  saveTasksToStorage()
}

function completeCurrentTask() {
  if (!currentTask.value) return

  const doneId = currentTask.value.id
  setTaskCompleted(doneId, true)

  isOvertime.value = false
  overtimeSeconds.value = 0

  const nextTask = activeTasks.value.find((item) => item.id !== doneId)
  if (nextTask) {
    startFocus(nextTask.id)
    return
  }

  pauseTimer()
  currentTaskId.value = null
  mode.value = 'timer'
  timerDuration.value = DEFAULT_TIMER_SECONDS
  timeLeft.value = DEFAULT_TIMER_SECONDS
  timeElapsed.value = 0
  currentView.value = 'dashboard'

  if (isMini.value) {
    void closeMiniWindowMode({ restoreMain: true })
  }
}

function toggleCompleted(id: number) {
  const task = tasks.value.find((item) => item.id === id)
  if (!task) return
  setTaskCompleted(id, !task.isCompleted)
}

function deleteTask(id: number) {
  tasks.value = tasks.value.filter((task) => task.id !== id)
  saveTasksToStorage()

  if (currentTaskId.value !== id) return

  pauseTimer()
  currentTaskId.value = null
  mode.value = 'timer'
  timerDuration.value = DEFAULT_TIMER_SECONDS
  timeLeft.value = DEFAULT_TIMER_SECONDS
  timeElapsed.value = 0
  isOvertime.value = false
  overtimeSeconds.value = 0
  currentView.value = 'dashboard'

  if (isMini.value) {
    void closeMiniWindowMode({ restoreMain: true })
  }
}

function buildRuntimeSnapshot() {
  return {
    mode: mode.value,
    isRunning: isRunning.value,
    isOvertime: isOvertime.value,
    displayTime: displayTime.value,
    statusText: statusText.value,
    progressPercentage: Math.max(0, Math.min(100, progressPercentage.value)),
    themeClass: themeClass.value,
    progressBarClass: progressBarClass.value,
    currentTaskTitle: currentTask.value?.title ?? null,
    hasCurrentTask: Boolean(currentTask.value),
  }
}

watchEffect(() => {
  const snapshot = buildRuntimeSnapshot()
  void emitPomodoroRuntimeSnapshot(snapshot)
})

async function handleMiniAction(action: PomodoroMiniAction) {
  switch (action.type) {
    case 'toggleTimer':
      toggleTimer()
      return
    case 'startBreak':
      startBreak(currentTask.value?.breakDuration || DEFAULT_BREAK_MINUTES)
      return
    case 'endBreak':
      endBreak()
      return
    case 'completeTask':
      completeCurrentTask()
      return
    case 'resetTimer':
      resetTimer()
      return
    case 'toggleMode':
      if (!currentTask.value && mode.value !== 'break') {
        toggleMode()
      }
      return
    case 'restoreMain':
      isMini.value = false
      currentView.value = 'focus'
      return
    default:
      return
  }
}

async function initMiniBridge() {
  if (miniActionUnlisten) return

  try {
    miniActionUnlisten = await listenPomodoroMiniAction(async (action) => {
      await handleMiniAction(action)
    })
  } catch (error) {
    console.error('初始化 mini 通信失败:', error)
  }
}

void initMiniBridge()

export {
  activeTasks,
  backToDashboard,
  completeCurrentTask,
  completedTasks,
  createTask,
  currentTask,
  currentView,
  deleteTask,
  displayTime,
  endBreak,
  enterMiniWindowMode,
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
  timeElapsed,
  timeLeft,
  timerDuration,
  toggleCompleted,
  toggleMode,
  toggleTimer,
  updateTask,
}
