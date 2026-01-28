<script setup lang="ts">
import { ref, computed } from 'vue'
import * as Diff from 'diff'
import { NInput, NButton, NSelect, NTag, NScrollbar } from 'naive-ui'
import { Clean, Compare } from '@vicons/carbon'
import { NIcon } from 'naive-ui'

// 状态
const oldText = ref('')
const newText = ref('')
const diffMode = ref('chars') // 'lines' | 'chars'

// 模式选项
const modeOptions = [
  { label: '字符对比 (适合文章)', value: 'chars' },
  { label: '行级对比 (适合代码)', value: 'lines' },
]

// 核心：计算差异
// Diff 库返回一个对象数组: { value: string, added?: boolean, removed?: boolean }
const diffResult = computed(() => {
  if (!oldText.value && !newText.value) return []

  if (diffMode.value === 'lines') {
    // 强制追加换行符以避免最后一行合并显示的问题
    return Diff.diffLines(oldText.value, newText.value, { newlineIsToken: true })
  } else {
    return Diff.diffChars(oldText.value, newText.value)
  }
})

// 统计信息
const stats = computed(() => {
  let additions = 0
  let deletions = 0
  diffResult.value.forEach((part) => {
    if (part.added) additions++
    if (part.removed) deletions++
  })
  return { additions, deletions }
})

// 示例填充
const fillExample = () => {
  oldText.value = `const krate = {\n  version: "1.0.0",\n  features: ["Hash"]\n}`
  newText.value = `const krate = {\n  version: "2.0.0",\n  features: ["Hash", "Diff", "JSON"]\n}`
}

const clearAll = () => {
  oldText.value = ''
  newText.value = ''
}
</script>

<template>
  <div class="h-full flex flex-col space-y-4">
    <div class="flex items-center justify-between pb-4 border-b border-slate-700/50">
      <div class="flex items-center space-x-4">
        <div>
          <h2 class="text-2xl font-bold text-slate-100 tracking-tight">文本差异对比</h2>
          <p class="text-slate-500 text-sm mt-1">快速比对文本或代码变动</p>
        </div>
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

    <div
      class="bg-[#0F172A]/40 p-3 rounded-xl border border-slate-700/50 flex items-center justify-between"
    >
      <div class="flex items-center space-x-4">
        <div class="w-48">
          <n-select size="small" v-model:value="diffMode" :options="modeOptions" />
        </div>

        <div class="flex space-x-2" v-if="oldText || newText">
          <n-tag type="error" size="small" round :bordered="false">
            - {{ stats.deletions }} 处删除
          </n-tag>
          <n-tag type="success" size="small" round :bordered="false">
            + {{ stats.additions }} 处新增
          </n-tag>
        </div>
      </div>
    </div>

    <div class="flex-1 flex flex-col min-h-0 gap-4">
      <div class="h-1/3 grid grid-cols-2 gap-4">
        <div class="flex flex-col relative group">
          <n-input
            v-model:value="oldText"
            type="textarea"
            placeholder="粘贴旧文本..."
            class="flex-1 font-mono text-xs leading-relaxed !bg-slate-900/50 border-slate-700 hover:border-red-500/30 focus:border-red-500/50 rounded-xl p-2"
            :input-props="{ style: 'height: 100%;', spellcheck: false }"
          />
        </div>

        <div class="flex flex-col relative group">
          <n-input
            v-model:value="newText"
            type="textarea"
            placeholder="粘贴新文本..."
            class="flex-1 font-mono text-xs leading-relaxed !bg-slate-900/50 border-slate-700 hover:border-green-500/30 focus:border-green-500/50 rounded-xl p-2"
            :input-props="{ style: 'height: 100%;', spellcheck: false }"
          />
        </div>
      </div>

      <div class="flex-1 flex flex-col relative min-h-0">
        <div class="text-xs text-slate-500 mb-2 font-medium pl-1">DIFF RESULT</div>

        <div
          class="flex-1 bg-[#0F172A] border border-slate-700/50 rounded-xl overflow-hidden shadow-inner relative"
        >
          <n-scrollbar x-scrollable>
            <div class="p-4 font-mono text-sm min-w-max">
              <template v-if="diffResult.length > 0">
                <span
                  v-for="(part, index) in diffResult"
                  :key="index"
                  class="whitespace-pre-wrap break-all"
                  :class="{
                    'bg-green-500/20 text-green-300': part.added,
                    'bg-red-500/20 text-red-300 decoration-red-500/50 line-through decoration-2':
                      part.removed,
                    'text-slate-400': !part.added && !part.removed,
                  }"
                  >{{ part.value }}</span
                >
              </template>

              <div
                v-else
                class="h-full flex flex-col items-center justify-center text-slate-600 space-y-2 opacity-50 mt-10"
              >
                <n-icon :size="48" :component="Compare" />
                <span class="text-sm">暂无内容，请在上方输入</span>
              </div>
            </div>
          </n-scrollbar>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
:deep(.n-input) {
  background-color: transparent;
}
:deep(.n-input__textarea-el) {
  height: 100% !important;
  font-family: 'JetBrains Mono', 'Fira Code', Consolas, monospace;
}
</style>
