<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { NButton, NEmpty, NIcon, NInput, NTag, useMessage } from 'naive-ui'
import { Document, Search, Renew, Terminal } from '@vicons/carbon'

interface LogFileInfo {
  path: string
  size: number
  isBinary: boolean
}

interface LogChunkResponse {
  startOffset: number
  endOffset: number
  eof: boolean
  text: string
}

interface LogSearchMatch {
  offset: number
  preview: string
}

interface LogSearchResponse {
  matches: LogSearchMatch[]
  nextOffset: number
  done: boolean
}

interface LoadedChunk {
  startOffset: number
  endOffset: number
  lines: string[]
}

const message = useMessage()

const ROW_HEIGHT = 20
const MIN_CHUNK_BYTES = 8 * 1024
const CHUNK_BYTES = 1024 * 1024
const TAIL_BYTES = 2 * 1024 * 1024
const MAX_CACHED_CHUNKS = 8
const LOAD_THRESHOLD = 240

const filePath = ref('')
const fileSize = ref(0)
const isBinary = ref(false)
const isOpening = ref(false)
const isChunkLoading = ref(false)
const errorText = ref('')

const chunks = ref<LoadedChunk[]>([])
const scrollContainer = ref<HTMLElement | null>(null)
const scrollTop = ref(0)
const viewportHeight = ref(0)

const searchKeyword = ref('')
const searchResults = ref<LogSearchMatch[]>([])
const searchNextOffset = ref(0)
const searchDone = ref(false)
const isSearching = ref(false)

const allLines = computed(() => chunks.value.flatMap((chunk) => chunk.lines))
const lineCount = computed(() => allLines.value.length)
const earliestOffset = computed(() => chunks.value[0]?.startOffset ?? 0)
const latestOffset = computed(() => chunks.value[chunks.value.length - 1]?.endOffset ?? 0)
const cachedChunks = computed(() => chunks.value.length)
const cachedBytes = computed(() => Math.max(0, latestOffset.value - earliestOffset.value))
const hasFile = computed(() => !!filePath.value)
const isAtFileTop = computed(() => earliestOffset.value <= 0)
const isAtFileEnd = computed(() => latestOffset.value >= fileSize.value)
const searchProgressText = computed(() => {
  if (!hasFile.value || fileSize.value <= 0 || !searchKeyword.value.trim()) return '--'
  const percent = Math.min(100, (searchNextOffset.value / fileSize.value) * 100)
  return `${percent.toFixed(1)}%`
})

const visibleStart = computed(() => Math.max(0, Math.floor(scrollTop.value / ROW_HEIGHT) - 30))
const visibleEnd = computed(() =>
  Math.min(lineCount.value, visibleStart.value + Math.ceil(viewportHeight.value / ROW_HEIGHT) + 60),
)
const topPadding = computed(() => visibleStart.value * ROW_HEIGHT)
const bottomPadding = computed(() => Math.max(0, (lineCount.value - visibleEnd.value) * ROW_HEIGHT))
const visibleLines = computed(() =>
  allLines.value.slice(visibleStart.value, visibleEnd.value).map((line, idx) => ({
    index: visibleStart.value + idx,
    text: line,
  })),
)

const formatBytes = (bytes: number) => {
  if (!Number.isFinite(bytes) || bytes <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let value = bytes
  let unit = 0
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024
    unit += 1
  }
  return `${value.toFixed(value >= 100 || unit === 0 ? 0 : 2)} ${units[unit]}`
}

const clearState = () => {
  errorText.value = ''
  chunks.value = []
  scrollTop.value = 0
  searchResults.value = []
  searchNextOffset.value = 0
  searchDone.value = false
}

const toLines = (text: string) => text.replace(/\r\n/g, '\n').split('\n')

const toLoadedChunk = (chunk: LogChunkResponse): LoadedChunk => ({
  startOffset: chunk.startOffset,
  endOffset: chunk.endOffset,
  lines: toLines(chunk.text),
})

const updateViewport = () => {
  if (scrollContainer.value) {
    viewportHeight.value = scrollContainer.value.clientHeight
  }
}

const readChunk = async (path: string, offset: number, maxBytes: number) =>
  invoke<LogChunkResponse>('read_log_chunk', {
    path,
    offset,
    max_bytes: maxBytes,
  })

const readTail = async (path: string, windowBytes: number) =>
  invoke<LogChunkResponse>('read_log_tail', {
    path,
    window_bytes: windowBytes,
  })

const appendChunk = async (chunk: LogChunkResponse) => {
  if (chunk.endOffset <= chunk.startOffset) return
  const loaded = toLoadedChunk(chunk)
  chunks.value.push(loaded)

  if (chunks.value.length > MAX_CACHED_CHUNKS) {
    const removed = chunks.value.shift()
    if (removed && scrollContainer.value) {
      const delta = removed.lines.length * ROW_HEIGHT
      scrollContainer.value.scrollTop = Math.max(0, scrollContainer.value.scrollTop - delta)
      scrollTop.value = scrollContainer.value.scrollTop
    }
  }
}

const prependChunk = async (chunk: LogChunkResponse) => {
  if (chunk.endOffset <= chunk.startOffset) return
  const loaded = toLoadedChunk(chunk)
  chunks.value.unshift(loaded)
  await nextTick()

  if (scrollContainer.value) {
    const delta = loaded.lines.length * ROW_HEIGHT
    scrollContainer.value.scrollTop += delta
    scrollTop.value = scrollContainer.value.scrollTop
  }

  if (chunks.value.length > MAX_CACHED_CHUNKS) {
    chunks.value.pop()
  }
}

const scrollToBottom = async () => {
  await nextTick()
  if (scrollContainer.value) {
    scrollContainer.value.scrollTop = scrollContainer.value.scrollHeight
    scrollTop.value = scrollContainer.value.scrollTop
  }
}

const openLogFile = async (path: string) => {
  isOpening.value = true
  clearState()

  try {
    const info = await invoke<LogFileInfo>('inspect_log_file', { path })
    filePath.value = info.path
    fileSize.value = info.size
    isBinary.value = info.isBinary

    if (info.isBinary) {
      errorText.value = '当前文件被识别为二进制文件，不支持文本预览。'
      message.warning(errorText.value)
      return
    }

    const tail = await readTail(info.path, TAIL_BYTES)
    chunks.value = [toLoadedChunk(tail)]
    await scrollToBottom()
    updateViewport()
  } catch (error: any) {
    errorText.value = error?.toString?.() || '打开文件失败'
    message.error(`打开失败: ${errorText.value}`)
  } finally {
    isOpening.value = false
  }
}

const chooseFile = async () => {
  const selected = await open({
    multiple: false,
    title: '选择日志文件',
  })
  if (typeof selected !== 'string') return
  await openLogFile(selected)
}

const loadNextChunk = async () => {
  if (!hasFile.value || isChunkLoading.value || isAtFileEnd.value || isBinary.value) return
  isChunkLoading.value = true

  try {
    const chunk = await readChunk(filePath.value, latestOffset.value, CHUNK_BYTES)
    await appendChunk(chunk)
  } catch (error: any) {
    message.error(`读取失败: ${error?.toString?.() || error}`)
  } finally {
    isChunkLoading.value = false
  }
}

const loadPrevChunk = async () => {
  if (!hasFile.value || isChunkLoading.value || isAtFileTop.value || isBinary.value) return
  isChunkLoading.value = true

  try {
    const start = Math.max(0, earliestOffset.value - CHUNK_BYTES)
    const length = Math.max(MIN_CHUNK_BYTES, earliestOffset.value - start)
    const chunk = await readChunk(filePath.value, start, length)
    await prependChunk(chunk)
  } catch (error: any) {
    message.error(`读取失败: ${error?.toString?.() || error}`)
  } finally {
    isChunkLoading.value = false
  }
}

const handleScroll = (event: Event) => {
  const target = event.target as HTMLElement
  scrollTop.value = target.scrollTop

  if (target.scrollTop <= LOAD_THRESHOLD) {
    void loadPrevChunk()
  }
  if (target.scrollTop + target.clientHeight >= target.scrollHeight - LOAD_THRESHOLD) {
    void loadNextChunk()
  }
}

const escapeHtml = (value: string) =>
  value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')

const highlightedLine = (line: string) => {
  const source = line.length ? line : ' '
  const term = searchKeyword.value.trim()
  if (!term) return escapeHtml(source)

  const sourceLower = source.toLowerCase()
  const termLower = term.toLowerCase()
  let cursor = 0
  let out = ''

  while (cursor < source.length) {
    const idx = sourceLower.indexOf(termLower, cursor)
    if (idx < 0) {
      out += escapeHtml(source.slice(cursor))
      break
    }
    out += escapeHtml(source.slice(cursor, idx))
    out += `<mark class="bg-amber-500/35 text-amber-100 px-0.5 rounded">${escapeHtml(source.slice(idx, idx + term.length))}</mark>`
    cursor = idx + term.length
  }

  return out || '&nbsp;'
}

const searchInFile = async (reset: boolean) => {
  const keyword = searchKeyword.value.trim()
  if (!hasFile.value || !keyword || isBinary.value) return
  if (isSearching.value) return

  if (reset) {
    searchResults.value = []
    searchNextOffset.value = 0
    searchDone.value = false
  }
  if (searchDone.value) return

  isSearching.value = true
  try {
    const result = await invoke<LogSearchResponse>('search_log_in_file', {
      path: filePath.value,
      keyword,
      start_offset: searchNextOffset.value,
      limit: 80,
    })
    searchResults.value = [...searchResults.value, ...result.matches]
    searchNextOffset.value = result.nextOffset
    searchDone.value = result.done

    if (reset && result.matches.length > 0) {
      await jumpToOffset(result.matches[0].offset)
    }
  } catch (error: any) {
    message.error(`搜索失败: ${error?.toString?.() || error}`)
  } finally {
    isSearching.value = false
  }
}

const jumpToOffset = async (offset: number) => {
  if (!hasFile.value || isBinary.value) return

  const start = Math.max(0, offset - Math.floor(CHUNK_BYTES / 2))
  const chunk = await readChunk(filePath.value, start, CHUNK_BYTES)
  chunks.value = [toLoadedChunk(chunk)]

  await nextTick()
  updateViewport()

  if (!scrollContainer.value) return
  const keyword = searchKeyword.value.trim().toLowerCase()
  if (!keyword) {
    scrollContainer.value.scrollTop = 0
    scrollTop.value = 0
    return
  }

  const idx = allLines.value.findIndex((line) => line.toLowerCase().includes(keyword))
  const target = idx > 3 ? (idx - 3) * ROW_HEIGHT : 0
  scrollContainer.value.scrollTop = target
  scrollTop.value = target
}

onMounted(() => {
  window.addEventListener('resize', updateViewport)
  updateViewport()
})

onUnmounted(() => {
  window.removeEventListener('resize', updateViewport)
})
</script>

<template>
  <div class="log-viewer-page h-full flex flex-col p-6 gap-4 overflow-hidden">
    <section class="rounded-2xl border border-cyan-500/25 bg-gradient-to-r from-slate-900/95 via-slate-900/70 to-cyan-900/20 p-4 shadow-[0_10px_30px_rgba(8,47,73,0.28)]">
      <div class="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
        <div class="space-y-2">
          <div class="flex items-center gap-2 text-cyan-300">
            <n-icon :component="Terminal" />
            <span class="text-xs tracking-[0.24em] uppercase">Log Viewer</span>
          </div>
          <h2 class="text-2xl font-semibold text-slate-100">大文件日志查看</h2>
          <p class="text-sm text-slate-400">按需加载 + 虚拟滚动，目标是 GB 级文本文件依然流畅。</p>
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <n-tag type="default" :bordered="false">文件大小 {{ formatBytes(fileSize) }}</n-tag>
          <n-tag v-if="hasFile && !isBinary" type="info" :bordered="false">
            缓存窗口 {{ formatBytes(cachedBytes) }}
          </n-tag>
          <n-tag v-if="isAtFileTop && hasFile && !isBinary" type="success" :bordered="false">已到开头</n-tag>
          <n-tag v-if="isAtFileEnd && hasFile && !isBinary" type="success" :bordered="false">已到末尾</n-tag>
          <n-button type="primary" :loading="isOpening" @click="chooseFile">
            <template #icon>
              <n-icon :component="Document" />
            </template>
            选择日志文件
          </n-button>
        </div>
      </div>
      <div class="mt-3 text-xs text-slate-400 truncate">当前文件: {{ filePath || '未选择' }}</div>
    </section>

    <section class="rounded-xl border border-slate-700/80 bg-slate-900/70 p-3">
      <div class="flex flex-col gap-2 lg:flex-row">
        <n-input
          v-model:value="searchKeyword"
          placeholder="输入关键词，回车立即搜索并高亮"
          clearable
          @keydown.enter.prevent="searchInFile(true)"
        />
        <div class="flex items-center gap-2">
          <n-button type="primary" :loading="isSearching" @click="searchInFile(true)">
            <template #icon><n-icon :component="Search" /></template>
            搜索
          </n-button>
          <n-button secondary :disabled="searchDone || isSearching || !searchKeyword.trim()" @click="searchInFile(false)">
            加载更多结果
          </n-button>
          <n-button tertiary :disabled="!hasFile || isBinary" @click="openLogFile(filePath)">
            <template #icon><n-icon :component="Renew" /></template>
            重新加载
          </n-button>
        </div>
      </div>
    </section>

    <div class="flex-1 min-h-0 grid grid-cols-1 xl:grid-cols-[minmax(0,1fr)_340px] gap-4">
      <section class="min-h-0 rounded-2xl border border-slate-700/80 bg-slate-950/80 overflow-hidden">
        <div v-if="!hasFile" class="h-full flex items-center justify-center">
          <n-empty description="请选择一个日志文件开始查看" />
        </div>

        <div v-else-if="isBinary" class="h-full flex items-center justify-center text-slate-300 px-6 text-center">
          {{ errorText || '该文件被识别为二进制文件，无法按文本方式显示。' }}
        </div>

        <div v-else class="h-full flex flex-col min-h-0">
          <div class="grid grid-cols-2 md:grid-cols-4 gap-2 px-3 py-2 border-b border-slate-700/80 text-xs">
            <div class="panel-kv">
              <span>缓存行数</span>
              <strong>{{ lineCount.toLocaleString() }}</strong>
            </div>
            <div class="panel-kv">
              <span>缓存分块</span>
              <strong>{{ cachedChunks }}</strong>
            </div>
            <div class="panel-kv">
              <span>起始偏移</span>
              <strong>{{ earliestOffset.toLocaleString() }}</strong>
            </div>
            <div class="panel-kv">
              <span>结束偏移</span>
              <strong>{{ latestOffset.toLocaleString() }}</strong>
            </div>
          </div>

          <div
            ref="scrollContainer"
            class="log-scroll flex-1 overflow-auto font-mono text-xs leading-5"
            @scroll.passive="handleScroll"
          >
            <div :style="{ height: `${topPadding}px` }"></div>
            <div
              v-for="row in visibleLines"
              :key="row.index"
              class="log-line h-5 min-w-max"
            >
              <div class="log-line-no">{{ row.index + 1 }}</div>
              <div class="log-line-text" v-html="highlightedLine(row.text)"></div>
            </div>
            <div :style="{ height: `${bottomPadding}px` }"></div>
          </div>
        </div>
      </section>

      <aside class="min-h-0 rounded-2xl border border-slate-700/80 bg-slate-900/75 overflow-hidden flex flex-col">
        <div class="px-3 py-2 border-b border-slate-700/80">
          <div class="flex items-center justify-between text-sm">
            <span class="text-slate-100 font-medium">搜索结果</span>
            <span class="text-slate-400">{{ searchResults.length }} 条</span>
          </div>
          <div class="mt-1 text-xs text-slate-500">
            扫描进度: {{ searchProgressText }}{{ searchDone ? ' · 已到末尾' : '' }}
          </div>
        </div>

        <div v-if="searchResults.length === 0" class="flex-1 flex items-center justify-center text-slate-500 text-sm px-4 text-center">
          输入关键词后执行搜索，命中结果可点击跳转。
        </div>

        <div v-else class="flex-1 overflow-auto p-2 space-y-2">
          <button
            v-for="item in searchResults"
            :key="item.offset"
            class="search-hit w-full text-left px-2 py-2 rounded-lg border border-slate-700/70 bg-slate-900/75 hover:border-cyan-500/60 hover:bg-slate-800/90 transition-colors"
            @click="jumpToOffset(item.offset)"
          >
            <div class="text-cyan-300 font-mono text-xs mb-1">offset {{ item.offset.toLocaleString() }}</div>
            <div class="text-slate-200 font-mono text-xs leading-5 break-all" v-html="highlightedLine(item.preview)"></div>
          </button>
        </div>
      </aside>
    </div>
  </div>
</template>

<style scoped>
.log-viewer-page {
  --line-no-width: 78px;
}

.panel-kv {
  background: rgba(15, 23, 42, 0.54);
  border: 1px solid rgba(71, 85, 105, 0.4);
  border-radius: 10px;
  padding: 6px 10px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.panel-kv > span {
  color: rgb(148 163 184);
}

.panel-kv > strong {
  color: rgb(226 232 240);
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace;
  font-weight: 600;
}

.log-scroll {
  background-image: linear-gradient(180deg, rgba(15, 23, 42, 0.44) 0%, rgba(2, 6, 23, 0.92) 100%);
}

.log-line {
  display: flex;
  align-items: center;
  border-bottom: 1px solid rgba(30, 41, 59, 0.42);
  font-variant-ligatures: none;
}

.log-line:hover {
  background: rgba(8, 47, 73, 0.25);
}

.log-line-no {
  width: var(--line-no-width);
  flex-shrink: 0;
  text-align: right;
  padding-right: 12px;
  color: rgb(100 116 139);
  border-right: 1px solid rgba(51, 65, 85, 0.6);
}

.log-line-text {
  padding-left: 12px;
  color: rgb(226 232 240);
  white-space: pre;
}

.search-hit:focus-visible {
  outline: 2px solid rgba(34, 211, 238, 0.75);
  outline-offset: 1px;
}
</style>
