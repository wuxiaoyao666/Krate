<script setup lang="ts">
import { ref, computed } from 'vue'
import * as Diff from 'diff'
import { NInput, NButton, NTag, NScrollbar } from 'naive-ui'
import { Clean, Compare } from '@vicons/carbon'
import { NIcon } from 'naive-ui'

type Seg = { text: string; cls?: string }
type RowType = 'context' | 'add' | 'del' | 'change'

type DiffRow = {
  type: RowType
  oldNo: number | null
  newNo: number | null
  oldSegs: Seg[]
  newSegs: Seg[]
}

// 状态
const oldText = ref('')
const newText = ref('')

// 示例填充
const fillExample = () => {
  oldText.value = `const krate = {
  version: "1.0.0",
  features: ["Hash"]
}
`
  newText.value = `const krate = {
  version: "2.0.0",
  features: ["Hash", "Diff", "JSON"]
}
`
}

const clearAll = () => {
  oldText.value = ''
  newText.value = ''
}

// ---- 工具函数 ----
const normalizeForLineDiff = (s: string) => (s.endsWith('\n') ? s : s + '\n')

const splitLines = (s: string) => {
  // diffLines 的 value 经常以 '\n' 结尾，这里去掉最后一个空项，保证渲染不多一行
  const arr = s.split('\n')
  if (arr.length && arr[arr.length - 1] === '') arr.pop()
  return arr
}

const mkPlainSeg = (text: string): Seg[] => [{ text }]

// 行内高亮：把 old/new 的共同部分保留，差异部分分别高亮到两侧
const intraline = (oldLine: string, newLine: string) => {
  const parts = Diff.diffWordsWithSpace(oldLine, newLine)
  const left: Seg[] = []
  const right: Seg[] = []

  for (const p of parts as any[]) {
    if (p.added) {
      right.push({
        text: p.value,
        cls: 'bg-green-500/25 text-green-200 rounded px-0.5',
      })
    } else if (p.removed) {
      left.push({
        text: p.value,
        cls: 'bg-red-500/25 text-red-200 rounded px-0.5',
      })
    } else {
      left.push({ text: p.value })
      right.push({ text: p.value })
    }
  }

  return { left, right }
}

// ---- 核心：生成 GitHub Split Diff 行 ----
const diffRows = computed<DiffRow[]>(() => {
  const a = normalizeForLineDiff(oldText.value || '')
  const b = normalizeForLineDiff(newText.value || '')

  // 空内容直接返回空，避免显示一堆“空行 diff”
  if (!oldText.value && !newText.value) return []

  const parts = Diff.diffLines(a, b) as any[]

  const rows: DiffRow[] = []
  let oldNo = 1
  let newNo = 1

  for (let i = 0; i < parts.length; i++) {
    const part = parts[i]

    // 处理 “删除块 + 紧跟新增块” 作为 change（更贴近 GitHub 的修改体验）
    if (part.removed && parts[i + 1]?.added) {
      const removedLines = splitLines(part.value)
      const addedLines = splitLines(parts[i + 1].value)
      const max = Math.max(removedLines.length, addedLines.length)

      for (let j = 0; j < max; j++) {
        const hasOld = j < removedLines.length
        const hasNew = j < addedLines.length

        const oldLine = hasOld ? removedLines[j] : ''
        const newLine = hasNew ? addedLines[j] : ''

        if (hasOld && hasNew) {
          const hi = intraline(oldLine, newLine)
          rows.push({
            type: 'change',
            oldNo: oldNo++,
            newNo: newNo++,
            oldSegs: hi.left.length ? hi.left : mkPlainSeg(oldLine),
            newSegs: hi.right.length ? hi.right : mkPlainSeg(newLine),
          })
        } else if (hasOld) {
          rows.push({
            type: 'del',
            oldNo: oldNo++,
            newNo: null,
            oldSegs: mkPlainSeg(oldLine),
            newSegs: mkPlainSeg(''),
          })
        } else {
          rows.push({
            type: 'add',
            oldNo: null,
            newNo: newNo++,
            oldSegs: mkPlainSeg(''),
            newSegs: mkPlainSeg(newLine),
          })
        }
      }

      i++ // 跳过下一段 added
      continue
    }

    // 普通 added / removed / context
    const lines = splitLines(part.value)

    if (part.added) {
      for (const line of lines) {
        rows.push({
          type: 'add',
          oldNo: null,
          newNo: newNo++,
          oldSegs: mkPlainSeg(''),
          newSegs: mkPlainSeg(line),
        })
      }
    } else if (part.removed) {
      for (const line of lines) {
        rows.push({
          type: 'del',
          oldNo: oldNo++,
          newNo: null,
          oldSegs: mkPlainSeg(line),
          newSegs: mkPlainSeg(''),
        })
      }
    } else {
      for (const line of lines) {
        rows.push({
          type: 'context',
          oldNo: oldNo++,
          newNo: newNo++,
          oldSegs: mkPlainSeg(line),
          newSegs: mkPlainSeg(line),
        })
      }
    }
  }

  return rows
})

// 统计
const stats = computed(() => {
  let additions = 0
  let deletions = 0

  for (const r of diffRows.value) {
    if (r.type === 'add') additions++
    if (r.type === 'del') deletions++
    if (r.type === 'change') {
      additions++
      deletions++
    }
  }
  return { additions, deletions }
})
</script>

<template>
  <div class="h-full flex flex-col space-y-4">
    <div class="flex items-center justify-between pb-4 border-b border-slate-700/50">
      <div>
        <h2 class="text-2xl font-bold text-slate-100 tracking-tight">文本差异对比</h2>
      </div>

      <div class="flex items-center space-x-3">
        <n-button quaternary size="small" @click="fillExample">示例</n-button>
        <div class="h-4 w-[1px] bg-slate-700"></div>
        <n-button quaternary size="small" @click="clearAll">
          <template #icon><n-icon :component="Clean" /></template>
          清空
        </n-button>
      </div>
    </div>

    <!-- Stats bar -->
    <div
      class="bg-[#0F172A]/40 p-3 rounded-xl border border-slate-700/50 flex items-center justify-between"
    >
      <div class="flex items-center space-x-3" v-if="oldText || newText">
        <n-tag type="error" size="small" round :bordered="false">
          - {{ stats.deletions }} 行删除
        </n-tag>
        <n-tag type="success" size="small" round :bordered="false">
          + {{ stats.additions }} 行新增
        </n-tag>
      </div>
      <div v-else class="text-slate-600 text-sm">输入文本后自动生成 GitHub 风格 Diff</div>
    </div>

    <div class="flex-1 flex flex-col min-h-0 gap-4">
      <!-- Inputs -->
      <div class="h-1/3 grid grid-cols-2 gap-4 min-h-0">
        <div class="flex flex-col min-h-0">
          <div class="text-xs text-slate-500 mb-2 font-medium pl-1">旧文本</div>
          <n-input
            v-model:value="oldText"
            type="textarea"
            placeholder="粘贴旧文本..."
            class="flex-1 font-mono text-xs leading-relaxed !bg-slate-900/50 border-slate-700 rounded-xl p-2"
            :input-props="{ style: 'height: 100%;', spellcheck: false }"
          />
        </div>

        <div class="flex flex-col min-h-0">
          <div class="text-xs text-slate-500 mb-2 font-medium pl-1">新文本</div>
          <n-input
            v-model:value="newText"
            type="textarea"
            placeholder="粘贴新文本..."
            class="flex-1 font-mono text-xs leading-relaxed !bg-slate-900/50 border-slate-700 rounded-xl p-2"
            :input-props="{ style: 'height: 100%;', spellcheck: false }"
          />
        </div>
      </div>

      <!-- Diff -->
      <div class="flex-1 flex flex-col relative min-h-0">
        <div class="text-xs text-slate-500 mb-2 font-medium pl-1">比对结果</div>

        <div
          class="flex-1 bg-[#0F172A] border border-slate-700/50 rounded-xl overflow-hidden shadow-inner relative min-h-0"
        >
          <n-scrollbar x-scrollable>
            <div v-if="diffRows.length" class="min-w-[900px]">
              <!-- 表头（仿 GitHub split） -->
              <div
                class="grid grid-cols-[64px_minmax(0,1fr)_64px_minmax(0,1fr)] text-xs text-slate-500 border-b border-slate-700/50"
              >
                <div class="px-3 py-2 bg-slate-900/30">旧</div>
                <div class="px-3 py-2 bg-slate-900/30">内容</div>
                <div class="px-3 py-2 bg-slate-900/30">新</div>
                <div class="px-3 py-2 bg-slate-900/30">内容</div>
              </div>

              <!-- 行 -->
              <div
                v-for="(r, idx) in diffRows"
                :key="idx"
                class="grid grid-cols-[64px_minmax(0,1fr)_64px_minmax(0,1fr)] text-sm font-mono"
              >
                <!-- old line no -->
                <div
                  class="px-3 py-1 text-right select-none border-r border-slate-700/30"
                  :class="{
                    'bg-red-500/10 text-red-300': r.type === 'del' || r.type === 'change',
                    'bg-slate-900/20 text-slate-500': r.type === 'context' || r.type === 'add',
                  }"
                >
                  {{ r.oldNo ?? '' }}
                </div>

                <!-- old code -->
                <div
                  class="px-3 py-1 whitespace-pre break-words border-r border-slate-700/30"
                  :class="{
                    'bg-red-500/10 text-red-200': r.type === 'del' || r.type === 'change',
                    'bg-slate-900/10 text-slate-400': r.type === 'context',
                    'bg-slate-900/10 text-slate-600': r.type === 'add',
                  }"
                >
                  <template v-if="r.oldSegs.length">
                    <span v-for="(s, i) in r.oldSegs" :key="i" :class="s.cls">{{ s.text }}</span>
                  </template>
                </div>

                <!-- new line no -->
                <div
                  class="px-3 py-1 text-right select-none border-r border-slate-700/30"
                  :class="{
                    'bg-green-500/10 text-green-300': r.type === 'add' || r.type === 'change',
                    'bg-slate-900/20 text-slate-500': r.type === 'context' || r.type === 'del',
                  }"
                >
                  {{ r.newNo ?? '' }}
                </div>

                <!-- new code -->
                <div
                  class="px-3 py-1 whitespace-pre break-words"
                  :class="{
                    'bg-green-500/10 text-green-200': r.type === 'add' || r.type === 'change',
                    'bg-slate-900/10 text-slate-400': r.type === 'context',
                    'bg-slate-900/10 text-slate-600': r.type === 'del',
                  }"
                >
                  <template v-if="r.newSegs.length">
                    <span v-for="(s, i) in r.newSegs" :key="i" :class="s.cls">{{ s.text }}</span>
                  </template>
                </div>
              </div>
            </div>

            <!-- empty -->
            <div
              v-else
              class="h-full flex flex-col items-center justify-center text-slate-600 space-y-2 opacity-50 mt-10"
            >
              <n-icon :size="48" :component="Compare" />
              <span class="text-sm">暂无内容，请在上方输入</span>
            </div>
          </n-scrollbar>
        </div>
      </div>
    </div>
  </div>
</template>
