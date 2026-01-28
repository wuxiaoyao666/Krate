<script setup lang="ts">
import { ref, watch } from 'vue'
import { NInput, NButton, useMessage, NIcon, NSelect } from 'naive-ui'
import { Copy, Clean, Code, CenterToFit } from '@vicons/carbon'

const message = useMessage()

// 状态
const inputContent = ref('')
const outputContent = ref('')
const indentSize = ref<number | string>(2) // 默认 2 空格缩进
const errorMsg = ref('')

// 缩进选项
const indentOptions: Array<{ label: string; value: number | string }> = [
  { label: '2 空格缩进', value: 2 },
  { label: '4 空格缩进', value: 4 },
  { label: 'Tab 缩进', value: '\t' },
]

// 用户选择缩进时自动格式化
watch(indentSize, () => {
  if (!inputContent.value) return

  formatJSON()
})

// 核心：格式化
const formatJSON = () => {
  if (!inputContent.value) return
  errorMsg.value = ''

  try {
    const obj = JSON.parse(inputContent.value)
    // @ts-ignore
    outputContent.value = JSON.stringify(obj, null, indentSize.value)
  } catch (err: any) {
    console.error(err)
    errorMsg.value = `格式错误: ${err.message}`
    outputContent.value = ''
  }
}

// 压缩
const minifyJSON = () => {
  if (!inputContent.value) return
  errorMsg.value = ''

  try {
    const obj = JSON.parse(inputContent.value)
    outputContent.value = JSON.stringify(obj)
  } catch (err: any) {
    errorMsg.value = `格式错误: ${err.message}`
  }
}

// 复制
const copyResult = async () => {
  if (outputContent.value) {
    await navigator.clipboard.writeText(outputContent.value)
    message.success('已复制到剪贴板')
  }
}

// 清空
const clearAll = () => {
  inputContent.value = ''
  outputContent.value = ''
  errorMsg.value = ''
}

// 示例填充
const fillExample = () => {
  inputContent.value =
    '{"name":"Krate","version":"1.0.0","features":["Hash","JSON"],"author":{"name":"User","active":true}}'
  formatJSON()
}
</script>

<template>
  <div class="h-full flex flex-col space-y-4">
    <div class="flex items-center justify-between pb-4 border-b border-slate-700/50">
      <div class="flex items-center space-x-4">
        <div>
          <h2 class="text-2xl font-bold text-slate-100 tracking-tight">JSON 工具</h2>
          <p class="text-slate-500 text-sm mt-1">格式化、压缩与校验</p>
        </div>
      </div>

      <div class="flex items-center space-x-3">
        <n-button quaternary size="small" @click="fillExample"> 示例 </n-button>
        <div class="h-4 w-[1px] bg-slate-700"></div>
        <n-button quaternary size="small" @click="clearAll">
          <template #icon><n-icon :component="Clean" /></template>
          清空
        </n-button>
      </div>
    </div>

    <div
      class="bg-[#0F172A]/40 p-3 rounded-xl border border-slate-700/50 flex flex-wrap gap-4 items-center justify-between"
    >
      <div class="w-40">
        <n-select size="small" v-model:value="indentSize" :options="indentOptions as any" />
      </div>

      <div class="flex space-x-3">
        <n-button secondary type="primary" @click="minifyJSON">
          <template #icon><n-icon :component="CenterToFit" /></template>
          压缩
        </n-button>
        <n-button type="primary" @click="formatJSON">
          <template #icon><n-icon :component="Code" /></template>
          格式化
        </n-button>
      </div>
    </div>

    <div
      v-if="errorMsg"
      class="bg-red-500/10 border border-red-500/30 text-red-400 px-4 py-2 rounded-lg text-sm font-mono flex items-center"
    >
      <div class="w-2 h-2 bg-red-500 rounded-full mr-3 animate-pulse"></div>
      {{ errorMsg }}
    </div>

    <div class="flex-1 grid grid-cols-1 md:grid-cols-2 gap-4 min-h-0">
      <div class="flex flex-col h-full relative group">
        <div
          class="absolute top-3 right-4 z-10 text-xs text-slate-500 font-mono pointer-events-none"
        >
          INPUT
        </div>
        <n-input
          v-model:value="inputContent"
          type="textarea"
          placeholder="在此粘贴 JSON 字符串..."
          class="flex-1 font-mono text-sm leading-relaxed !bg-slate-900/50 border-slate-700 hover:border-cyan-500/50 focus:border-cyan-500 rounded-xl p-2"
          :input-props="{ style: 'height: 100%;', spellcheck: false }"
        />
      </div>

      <div class="flex flex-col h-full relative group">
        <div
          class="absolute top-3 right-4 z-10 text-xs text-cyan-500/50 font-mono pointer-events-none"
        >
          OUTPUT
        </div>
        <div class="flex-1 relative">
          <n-input
            v-model:value="outputContent"
            type="textarea"
            readonly
            placeholder="结果..."
            class="h-full font-mono text-sm leading-relaxed !bg-[#0F172A] border-cyan-900/30 text-cyan-400 rounded-xl p-2 shadow-inner"
            :input-props="{ style: 'height: 100%; resize: none;', spellcheck: false }"
          />

          <div class="absolute bottom-4 right-4" v-if="outputContent">
            <n-button
              type="primary"
              size="large"
              @click="copyResult"
              class="shadow-lg shadow-cyan-500/20"
            >
              <template #icon><n-icon :component="Copy" /></template>
              复制
            </n-button>
          </div>
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
