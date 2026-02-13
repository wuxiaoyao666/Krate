import { emitTo, listen } from '@tauri-apps/api/event'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { PhysicalPosition } from '@tauri-apps/api/dpi'
import { getCurrentWindow, monitorFromPoint, primaryMonitor } from '@tauri-apps/api/window'
import type { RunningMode } from './pomodoroTypes'

export const POMODORO_MAIN_WINDOW_LABEL = 'main'
export const POMODORO_MINI_WINDOW_LABEL = 'pomodoro-mini'
export const POMODORO_MINI_ROUTE = '/time/pomodoro-mini'

const POMODORO_STATE_EVENT = 'pomodoro://state'
const POMODORO_ACTION_EVENT = 'pomodoro://action'
const POMODORO_RUNTIME_STORAGE_KEY = 'krate.time.pomodoro.runtime'
const POMODORO_MINI_WINDOW_STATE_STORAGE_KEY = 'krate.time.pomodoro.mini.window'

const MINI_DEFAULT_WIDTH = 280
const MINI_DEFAULT_HEIGHT = 128
const MINI_MIN_WIDTH = 220
const MINI_MIN_HEIGHT = 100
const MINI_MAX_WIDTH = 480
const MINI_MAX_HEIGHT = 220

const MINI_EDGE_SNAP_DISTANCE = 22
const MINI_EVENT_DEBOUNCE_MS = 120

interface PomodoroMiniWindowState {
  width: number
  height: number
  x?: number
  y?: number
}

let miniMovedUnlisten: UnlistenFn | null = null
let miniResizedUnlisten: UnlistenFn | null = null
let miniDestroyedUnlisten: UnlistenFn | null = null

let miniPersistTimer: number | null = null
let miniSnapTimer: number | null = null
let miniApplyingSnap = false

export type PomodoroViewState = 'dashboard' | 'focus'

export interface PomodoroRuntimeSnapshot {
  mode: RunningMode
  isRunning: boolean
  isOvertime: boolean
  displayTime: string
  statusText: string
  progressPercentage: number
  themeClass: string
  progressBarClass: string
  currentTaskTitle: string | null
  hasCurrentTask: boolean
}

export type PomodoroMiniActionType =
  | 'toggleTimer'
  | 'startBreak'
  | 'endBreak'
  | 'completeTask'
  | 'resetTimer'
  | 'toggleMode'
  | 'restoreMain'

export interface PomodoroMiniAction {
  type: PomodoroMiniActionType
}

export const createEmptySnapshot = (): PomodoroRuntimeSnapshot => ({
  mode: 'timer',
  isRunning: false,
  isOvertime: false,
  displayTime: '25:00',
  statusText: '准备开始',
  progressPercentage: 100,
  themeClass: 'text-emerald-300',
  progressBarClass: 'bg-emerald-500',
  currentTaskTitle: null,
  hasCurrentTask: false,
})

function toFiniteNumber(value: unknown): number | null {
  const num = Number(value)
  return Number.isFinite(num) ? num : null
}

function clamp(value: number, min: number, max: number) {
  if (max < min) return min
  return Math.max(min, Math.min(max, value))
}

function clearMiniTimers() {
  if (miniPersistTimer !== null) {
    window.clearTimeout(miniPersistTimer)
    miniPersistTimer = null
  }

  if (miniSnapTimer !== null) {
    window.clearTimeout(miniSnapTimer)
    miniSnapTimer = null
  }
}

function cleanupMiniWindowBindings() {
  miniMovedUnlisten?.()
  miniMovedUnlisten = null
  miniResizedUnlisten?.()
  miniResizedUnlisten = null
  miniDestroyedUnlisten?.()
  miniDestroyedUnlisten = null
  clearMiniTimers()
  miniApplyingSnap = false
}

function readMiniWindowState(): PomodoroMiniWindowState {
  const fallback: PomodoroMiniWindowState = {
    width: MINI_DEFAULT_WIDTH,
    height: MINI_DEFAULT_HEIGHT,
  }

  if (typeof localStorage === 'undefined') return fallback

  const raw = localStorage.getItem(POMODORO_MINI_WINDOW_STATE_STORAGE_KEY)
  if (!raw) return fallback

  try {
    const parsed = JSON.parse(raw) as Partial<PomodoroMiniWindowState>
    const width = clamp(
      Math.floor(toFiniteNumber(parsed.width) ?? MINI_DEFAULT_WIDTH),
      MINI_MIN_WIDTH,
      MINI_MAX_WIDTH,
    )
    const height = clamp(
      Math.floor(toFiniteNumber(parsed.height) ?? MINI_DEFAULT_HEIGHT),
      MINI_MIN_HEIGHT,
      MINI_MAX_HEIGHT,
    )

    const x = toFiniteNumber(parsed.x)
    const y = toFiniteNumber(parsed.y)

    return {
      width,
      height,
      ...(x === null ? {} : { x: Math.round(x) }),
      ...(y === null ? {} : { y: Math.round(y) }),
    }
  } catch (error) {
    console.error('读取 mini 窗口状态失败:', error)
    return fallback
  }
}

function saveMiniWindowState(state: PomodoroMiniWindowState) {
  if (typeof localStorage === 'undefined') return
  localStorage.setItem(POMODORO_MINI_WINDOW_STATE_STORAGE_KEY, JSON.stringify(state))
}

async function persistMiniWindowState(mini: WebviewWindow) {
  try {
    const [position, size, scaleFactor] = await Promise.all([
      mini.outerPosition(),
      mini.outerSize(),
      mini.scaleFactor(),
    ])
    const safeScale = Number.isFinite(scaleFactor) && scaleFactor > 0 ? scaleFactor : 1

    saveMiniWindowState({
      x: Math.round(position.x / safeScale),
      y: Math.round(position.y / safeScale),
      width: clamp(Math.round(size.width / safeScale), MINI_MIN_WIDTH, MINI_MAX_WIDTH),
      height: clamp(Math.round(size.height / safeScale), MINI_MIN_HEIGHT, MINI_MAX_HEIGHT),
    })
  } catch (error) {
    console.error('保存 mini 窗口状态失败:', error)
  }
}

async function snapMiniWindowToEdges(mini: WebviewWindow) {
  if (miniApplyingSnap) return

  try {
    const [position, size] = await Promise.all([mini.outerPosition(), mini.outerSize()])
    const centerX = position.x + size.width / 2
    const centerY = position.y + size.height / 2
    const monitor =
      (await monitorFromPoint(centerX, centerY)) ?? (await primaryMonitor())

    if (!monitor) {
      await persistMiniWindowState(mini)
      return
    }

    const workAreaX = monitor.workArea.position.x
    const workAreaY = monitor.workArea.position.y
    const maxX = workAreaX + Math.max(0, monitor.workArea.size.width - size.width)
    const maxY = workAreaY + Math.max(0, monitor.workArea.size.height - size.height)

    let nextX = clamp(position.x, workAreaX, maxX)
    let nextY = clamp(position.y, workAreaY, maxY)

    if (Math.abs(nextX - workAreaX) <= MINI_EDGE_SNAP_DISTANCE) {
      nextX = workAreaX
    } else if (Math.abs(nextX - maxX) <= MINI_EDGE_SNAP_DISTANCE) {
      nextX = maxX
    }

    if (Math.abs(nextY - workAreaY) <= MINI_EDGE_SNAP_DISTANCE) {
      nextY = workAreaY
    } else if (Math.abs(nextY - maxY) <= MINI_EDGE_SNAP_DISTANCE) {
      nextY = maxY
    }

    if (nextX !== position.x || nextY !== position.y) {
      miniApplyingSnap = true
      try {
        await mini.setPosition(new PhysicalPosition(nextX, nextY))
      } finally {
        miniApplyingSnap = false
      }
    }

    await persistMiniWindowState(mini)
  } catch (error) {
    miniApplyingSnap = false
    console.error('mini 窗口边缘吸附失败:', error)
  }
}

function scheduleMiniWindowPersist(mini: WebviewWindow) {
  if (miniPersistTimer !== null) {
    window.clearTimeout(miniPersistTimer)
  }

  miniPersistTimer = window.setTimeout(() => {
    miniPersistTimer = null
    void persistMiniWindowState(mini)
  }, MINI_EVENT_DEBOUNCE_MS)
}

function scheduleMiniWindowSnap(mini: WebviewWindow) {
  if (miniSnapTimer !== null) {
    window.clearTimeout(miniSnapTimer)
  }

  miniSnapTimer = window.setTimeout(() => {
    miniSnapTimer = null
    void snapMiniWindowToEdges(mini)
  }, MINI_EVENT_DEBOUNCE_MS)
}

async function bindMiniWindowState(mini: WebviewWindow) {
  cleanupMiniWindowBindings()

  miniMovedUnlisten = await mini.onMoved(() => {
    if (miniApplyingSnap) return
    scheduleMiniWindowPersist(mini)
    scheduleMiniWindowSnap(mini)
  })

  miniResizedUnlisten = await mini.onResized(() => {
    scheduleMiniWindowPersist(mini)
    scheduleMiniWindowSnap(mini)
  })

  miniDestroyedUnlisten = await mini.once('tauri://destroyed', () => {
    cleanupMiniWindowBindings()
  })

  await persistMiniWindowState(mini)
}

export function readPomodoroRuntimeSnapshot(): PomodoroRuntimeSnapshot {
  if (typeof localStorage === 'undefined') {
    return createEmptySnapshot()
  }

  const raw = localStorage.getItem(POMODORO_RUNTIME_STORAGE_KEY)
  if (!raw) {
    return createEmptySnapshot()
  }

  try {
    return {
      ...createEmptySnapshot(),
      ...(JSON.parse(raw) as Partial<PomodoroRuntimeSnapshot>),
    }
  } catch (error) {
    console.error('读取番茄钟运行态失败:', error)
    return createEmptySnapshot()
  }
}

export async function emitPomodoroRuntimeSnapshot(snapshot: PomodoroRuntimeSnapshot) {
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(POMODORO_RUNTIME_STORAGE_KEY, JSON.stringify(snapshot))
  }

  try {
    await emitTo(POMODORO_MINI_WINDOW_LABEL, POMODORO_STATE_EVENT, snapshot)
    return
  } catch {
    // fallback: 使用窗口句柄发送，避免 emitTo 在部分平台失效
  }

  const mini = await WebviewWindow.getByLabel(POMODORO_MINI_WINDOW_LABEL)
  if (!mini) return

  try {
    await mini.emit(POMODORO_STATE_EVENT, snapshot)
  } catch {
    // mini 窗口未就绪时忽略
  }
}

export async function listenPomodoroRuntimeSnapshot(
  handler: (snapshot: PomodoroRuntimeSnapshot) => void,
): Promise<UnlistenFn> {
  return listen<PomodoroRuntimeSnapshot>(POMODORO_STATE_EVENT, (event) => {
    handler(event.payload)
  })
}

export async function emitPomodoroMiniAction(action: PomodoroMiniAction) {
  try {
    await emitTo(POMODORO_MAIN_WINDOW_LABEL, POMODORO_ACTION_EVENT, action)
    return
  } catch {
    // fallback: 直接发送到 main 窗口句柄
  }

  const mainWindow = await WebviewWindow.getByLabel(POMODORO_MAIN_WINDOW_LABEL)
  if (!mainWindow) return
  await mainWindow.emit(POMODORO_ACTION_EVENT, action)
}

export async function listenPomodoroMiniAction(
  handler: (action: PomodoroMiniAction) => void | Promise<void>,
): Promise<UnlistenFn> {
  return listen<PomodoroMiniAction>(POMODORO_ACTION_EVENT, async (event) => {
    await handler(event.payload)
  })
}

export async function openPomodoroMiniWindow() {
  const existing = await WebviewWindow.getByLabel(POMODORO_MINI_WINDOW_LABEL)
  if (existing) {
    await bindMiniWindowState(existing)
    await snapMiniWindowToEdges(existing)
    await existing.show()
    await existing.unminimize()
    await existing.setFocus()
    return existing
  }

  const restored = readMiniWindowState()
  const shouldCenter = restored.x === undefined || restored.y === undefined

  const mini = new WebviewWindow(POMODORO_MINI_WINDOW_LABEL, {
    url: POMODORO_MINI_ROUTE,
    title: 'Pomodoro Mini',
    width: restored.width,
    height: restored.height,
    minWidth: MINI_MIN_WIDTH,
    minHeight: MINI_MIN_HEIGHT,
    maxWidth: MINI_MAX_WIDTH,
    maxHeight: MINI_MAX_HEIGHT,
    resizable: true,
    decorations: false,
    transparent: true,
    shadow: false,
    alwaysOnTop: true,
    skipTaskbar: true,
    center: shouldCenter,
    ...(shouldCenter ? {} : { x: restored.x, y: restored.y }),
    focus: true,
  })

  await new Promise<void>((resolve, reject) => {
    mini.once('tauri://created', () => resolve())
    mini.once('tauri://error', (error) => reject(error))
  })

  await bindMiniWindowState(mini)
  await snapMiniWindowToEdges(mini)

  return mini
}

export async function closePomodoroMiniWindow() {
  const mini = await WebviewWindow.getByLabel(POMODORO_MINI_WINDOW_LABEL)
  if (!mini) {
    cleanupMiniWindowBindings()
    return
  }

  await mini.close()
  cleanupMiniWindowBindings()
}

export async function enterPomodoroMiniWindowMode() {
  await openPomodoroMiniWindow()
  const currentWindow = getCurrentWindow()

  try {
    // 触发主窗口 CloseRequested，实际由 Rust 拦截并隐藏到托盘
    await currentWindow.close()
  } catch {
    // fallback
    await currentWindow.hide()
  }
}

export async function restoreMainWindow() {
  const mainWindow = await WebviewWindow.getByLabel(POMODORO_MAIN_WINDOW_LABEL)
  if (!mainWindow) return

  await mainWindow.show()
  await mainWindow.unminimize()
  await mainWindow.setFocus()
}
