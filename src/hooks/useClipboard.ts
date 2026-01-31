import { ref, watch } from 'vue'
import { readText, writeText } from '@tauri-apps/plugin-clipboard-manager'

export interface HistoryItem {
  id: string
  text: string
  time: number
}

// === 全局单例状态 ===
const history = ref<HistoryItem[]>([])
// 默认关闭剪切板监听
const isListening = ref(false)

// 轮询重入锁
let inFlight = false

// 写回剪贴板后的短暂忽略
let ignoreText: string | null = null
let ignoreUntil = 0

// 保存防抖
let saveTimer: number | null = null

const MAX_ITEMS = 50
const MAX_TEXT_CHARS = 20_000
const SAVE_DEBOUNCE_MS = 200

// 定时器句柄
let pollTimer: number | null = null
let isInitialized = false

export function useClipboard() {
  // 加载数据
  const loadHistory = () => {
    const saved = localStorage.getItem('krate_clipboard_history')
    if (saved) {
      try {
        history.value = JSON.parse(saved)
      } catch (e) {}
    }
  }

  const normalizeText = (text: string) => {
    if (text.length <= MAX_TEXT_CHARS) return text
    return text.slice(0, MAX_TEXT_CHARS) + '\n…(已截断)'
  }

  // 保存数据
  const saveHistory = () => {
    if (saveTimer) return
    saveTimer = window.setTimeout(() => {
      saveTimer = null
      localStorage.setItem('krate_clipboard_history', JSON.stringify(history.value))
    }, SAVE_DEBOUNCE_MS)
  }

  // 加载设置
  const loadSetting = () => {
    const saved = localStorage.getItem('krate_clipboard_enabled')
    isListening.value = saved === 'true'
  }

  // 保存设置
  const saveSetting = () => {
    localStorage.setItem('krate_clipboard_enabled', String(isListening.value))
  }

  // 核心轮询任务
  const checkClipboard = async () => {
    if (!isListening.value) return
    if (inFlight) return

    inFlight = true
    try {
      const rawText = await readText()
      if (!rawText) return

      const text = normalizeText(rawText)

      if (
        !text ||
        (history.value.length > 0 && history.value[0].text === text) ||
        (text === ignoreText && Date.now() < ignoreUntil)
      ) {
        return
      }

      // LRU 去重策略
      const existingIndex = history.value.findIndex((i) => i.text === text)
      if (existingIndex !== -1) {
        history.value.splice(existingIndex, 1)
      }

      const newItem: HistoryItem = {
        id: crypto.randomUUID(),
        text: text, // 存入的是截断后的
        time: Date.now(),
      }

      history.value.unshift(newItem)

      if (history.value.length > MAX_ITEMS) {
        history.value.pop()
      }

      saveHistory()

    } finally {
      inFlight = false
    }
  }

  // 开启轮询
  const startPolling = () => {
    if (pollTimer) return // 已经开着了
    // 立即执行一次，不要等 1 秒
    checkClipboard()
    pollTimer = window.setInterval(checkClipboard, 1000)
    console.log('[Clipboard] Polling started')
  }

  // 停止轮询 (彻底释放定时器)
  const stopPolling = () => {
    if (pollTimer) {
      clearInterval(pollTimer)
      pollTimer = null
      console.log('[Clipboard] Polling stopped')
    }
  }

  // === 初始化 ===
  const initClipboard = () => {
    if (isInitialized) return
    isInitialized = true

    loadHistory()
    // 1. 读取配置
    loadSetting()

    // 2. 根据配置决定是否启动
    if (isListening.value) {
      startPolling()
    }

    // 3. 监听开关变化
    watch(isListening, (val) => {
      saveSetting() // 记住用户的选择
      if (val) {
        startPolling()
      } else {
        stopPolling()
      }
    })
  }

  // 复制并置顶
  const copyItem = async (item: HistoryItem) => {
    try {
      // 写回后 1.2s 内忽略这个文本
      ignoreText = item.text
      ignoreUntil = Date.now() + 1200

      await writeText(item.text)

      const index = history.value.findIndex((i) => i.id === item.id)
      if (index !== -1) {
        history.value.splice(index, 1)
        item.time = Date.now()
        history.value.unshift(item)
        saveHistory()
      }
      return true
    } catch {
      return false
    }
  }

  const clearHistory = () => {
    history.value = []
    saveHistory()
  }

  const deleteItem = (index: number) => {
    history.value.splice(index, 1)
    saveHistory()
  }

  return {
    history,
    isListening,
    initClipboard, // 改个名字，更语义化
    copyItem,
    clearHistory,
    deleteItem,
  }
}
