import { ref, watch } from 'vue'
import { readText, writeText } from '@tauri-apps/plugin-clipboard-manager'

export interface HistoryItem {
  id: string
  fullText: string
  previewText: string
  contentHash: string
  time: number
}

type StoredHistoryItem = Partial<HistoryItem> & { text?: string }

// 全局单例状态
const history = ref<HistoryItem[]>([])
const isListening = ref(false)

// 轮询重入锁
let inFlight = false

// 写回剪贴板后的短暂忽略
let ignoreText: string | null = null
let ignoreUntil = 0

// 保存防抖
let saveTimer: number | null = null

const MAX_ITEMS = 50
const PREVIEW_MAX_CHARS = 4000
const PREVIEW_SUFFIX = '\n...[已截断预览]'
const SAVE_DEBOUNCE_MS = 200

// 轮询定时器句柄
let pollTimer: number | null = null
let isInitialized = false

const hashText = (text: string) => {
  // 基于 UTF-16 码元的 64 位 FNV-1a，并拼接长度降低碰撞概率
  let hash = 0xcbf29ce484222325n
  for (let i = 0; i < text.length; i++) {
    hash ^= BigInt(text.charCodeAt(i))
    hash = BigInt.asUintN(64, hash * 0x100000001b3n)
  }
  return `${text.length}:${hash.toString(16).padStart(16, '0')}`
}

const buildPreviewText = (text: string) => {
  if (text.length <= PREVIEW_MAX_CHARS) return text
  return text.slice(0, PREVIEW_MAX_CHARS) + PREVIEW_SUFFIX
}

const normalizeStoredItem = (item: StoredHistoryItem): HistoryItem | null => {
  const fullText =
    typeof item.fullText === 'string'
      ? item.fullText
      : typeof item.text === 'string'
        ? item.text
        : ''

  if (!fullText) return null

  return {
    id: typeof item.id === 'string' && item.id.trim().length > 0 ? item.id : crypto.randomUUID(),
    fullText,
    previewText: typeof item.previewText === 'string' ? item.previewText : buildPreviewText(fullText),
    contentHash:
      typeof item.contentHash === 'string' && item.contentHash.length > 0
        ? item.contentHash
        : hashText(fullText),
    time: typeof item.time === 'number' ? item.time : Date.now(),
  }
}

export function useClipboard() {
  // 从 localStorage 加载历史，并迁移旧版数据结构
  const loadHistory = () => {
    const saved = localStorage.getItem('krate_clipboard_history')
    if (!saved) return

    try {
      const parsed = JSON.parse(saved)
      if (!Array.isArray(parsed)) {
        history.value = []
        return
      }

      history.value = parsed
        .map((item) => normalizeStoredItem(item ?? {}))
        .filter((item): item is HistoryItem => item !== null)
        .slice(0, MAX_ITEMS)
    } catch {
      history.value = []
    }
  }

  // 保存历史
  const saveHistory = () => {
    if (saveTimer) return
    saveTimer = window.setTimeout(() => {
      saveTimer = null
      try {
        localStorage.setItem('krate_clipboard_history', JSON.stringify(history.value))
      } catch (error) {
        console.warn('[Clipboard] 持久化历史失败:', error)
      }
    }, SAVE_DEBOUNCE_MS)
  }

  // 加载监听开关
  const loadSetting = () => {
    const saved = localStorage.getItem('krate_clipboard_enabled')
    isListening.value = saved === 'true'
  }

  // 保存监听开关
  const saveSetting = () => {
    localStorage.setItem('krate_clipboard_enabled', String(isListening.value))
  }

  // 核心轮询任务
  const checkClipboard = async () => {
    if (!isListening.value) return
    if (inFlight) return

    inFlight = true
    try {
      const fullText = await readText()
      if (!fullText) return

      if (
        (history.value.length > 0 && history.value[0].fullText === fullText) ||
        (fullText === ignoreText && Date.now() < ignoreUntil)
      ) {
        return
      }

      const contentHash = hashText(fullText)

      // 基于完整文本哈希做 LRU 去重，并用完整文本等值做兜底校验
      const existingIndex = history.value.findIndex(
        (item) => item.contentHash === contentHash && item.fullText === fullText,
      )
      if (existingIndex !== -1) {
        history.value.splice(existingIndex, 1)
      }

      const newItem: HistoryItem = {
        id: crypto.randomUUID(),
        fullText,
        previewText: buildPreviewText(fullText),
        contentHash,
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

  // 开始轮询
  const startPolling = () => {
    if (pollTimer) return
    checkClipboard()
    pollTimer = window.setInterval(checkClipboard, 1000)
    console.log('[Clipboard] 已启动轮询')
  }

  // 停止轮询
  const stopPolling = () => {
    if (pollTimer) {
      clearInterval(pollTimer)
      pollTimer = null
      console.log('[Clipboard] 已停止轮询')
    }
  }

  // 仅初始化一次
  const initClipboard = () => {
    if (isInitialized) return
    isInitialized = true

    loadHistory()
    loadSetting()

    if (isListening.value) {
      startPolling()
    }

    watch(isListening, (val) => {
      saveSetting()
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
      ignoreText = item.fullText
      ignoreUntil = Date.now() + 1200

      await writeText(item.fullText)

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
    initClipboard,
    copyItem,
    clearHistory,
    deleteItem,
  }
}
