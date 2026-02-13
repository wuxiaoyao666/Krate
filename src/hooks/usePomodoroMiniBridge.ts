import { emitTo, listen } from '@tauri-apps/api/event'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { RunningMode } from './pomodoroTypes'

export const POMODORO_MAIN_WINDOW_LABEL = 'main'
export const POMODORO_MINI_WINDOW_LABEL = 'pomodoro-mini'
export const POMODORO_MINI_ROUTE = '/time/pomodoro-mini'

const POMODORO_STATE_EVENT = 'pomodoro://state'
const POMODORO_ACTION_EVENT = 'pomodoro://action'
const POMODORO_RUNTIME_STORAGE_KEY = 'krate.time.pomodoro.runtime.v1'

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

export function readPomodoroRuntimeSnapshot(): PomodoroRuntimeSnapshot {
  if (typeof localStorage === 'undefined') {
    return createEmptySnapshot()
  }

  const raw = localStorage.getItem(POMODORO_RUNTIME_STORAGE_KEY)
  if (!raw) {
    return createEmptySnapshot()
  }

  try {
    const parsed = JSON.parse(raw) as Partial<PomodoroRuntimeSnapshot>
    return {
      ...createEmptySnapshot(),
      ...parsed,
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
    await existing.show()
    await existing.unminimize()
    await existing.setFocus()
    return existing
  }

  const mini = new WebviewWindow(POMODORO_MINI_WINDOW_LABEL, {
    url: POMODORO_MINI_ROUTE,
    title: 'Pomodoro Mini',
    width: 280,
    height: 128,
    minWidth: 220,
    minHeight: 100,
    maxWidth: 360,
    maxHeight: 180,
    resizable: false,
    decorations: false,
    transparent: true,
    shadow: false,
    alwaysOnTop: true,
    skipTaskbar: true,
    center: false,
    focus: true,
  })

  await new Promise<void>((resolve, reject) => {
    mini.once('tauri://created', () => resolve())
    mini.once('tauri://error', (error) => reject(error))
  })

  return mini
}

export async function closePomodoroMiniWindow() {
  const mini = await WebviewWindow.getByLabel(POMODORO_MINI_WINDOW_LABEL)
  if (!mini) return
  await mini.close()
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
