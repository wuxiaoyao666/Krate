<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { NButton, NSlider, NSwitch, NInputNumber, NProgress, useMessage } from 'naive-ui'
import { Copy, Renew, Clean } from '@vicons/carbon'
import { NIcon } from 'naive-ui'

const message = useMessage()

// 配置状态
const length = ref(16)
const includeUppercase = ref(true)
const includeLowercase = ref(true)
const includeNumbers = ref(true)
const includeSymbols = ref(true)
const excludeSimilar = ref(false) // 排除易混淆字符 (1, l, I, 0, O)

// 生成结果
const password = ref('')
const history = ref<string[]>([]) // 历史记录/批量展示

// 字符集定义
const CHAR_SETS = {
  upper: 'ABCDEFGHIJKLMNOPQRSTUVWXYZ',
  lower: 'abcdefghijklmnopqrstuvwxyz',
  number: '0123456789',
  symbol: '!@#$%^&*()_+~`|}{[]:;?><,./-=',
}

const SIMILAR_CHARS = /[Il1O0]/g

// 核心生成逻辑
const generate = () => {
  let chars = ''
  if (includeLowercase.value) chars += CHAR_SETS.lower
  if (includeUppercase.value) chars += CHAR_SETS.upper
  if (includeNumbers.value) chars += CHAR_SETS.number
  if (includeSymbols.value) chars += CHAR_SETS.symbol

  if (excludeSimilar.value) {
    chars = chars.replace(SIMILAR_CHARS, '')
  }

  // 如果什么都没选，默认选个小写，防止死循环
  if (!chars) {
    chars = CHAR_SETS.lower
    includeLowercase.value = true
    message.warning('至少需要选择一种字符类型，已自动重置')
  }

  let result = ''
  const array = new Uint32Array(length.value)
  window.crypto.getRandomValues(array) // 使用加密安全的随机数生成器

  for (let i = 0; i < length.value; i++) {
    result += chars[array[i] % chars.length]
  }

  password.value = result

  // 更新历史记录 (最新的在最前，保留最近5个)
  if (!history.value.includes(result)) {
    history.value.unshift(result)
    if (history.value.length > 5) history.value.pop()
  }
}

// 强度计算 (简单版)
const strength = computed(() => {
  let s = 0
  if (password.value.length > 8) s += 20
  if (password.value.length > 12) s += 20
  if (/[A-Z]/.test(password.value)) s += 15
  if (/[a-z]/.test(password.value)) s += 15
  if (/[0-9]/.test(password.value)) s += 15
  if (/[^A-Za-z0-9]/.test(password.value)) s += 15
  return Math.min(s, 100)
})

const strengthColor = computed(() => {
  if (strength.value < 40) return '#ef4444' // red
  if (strength.value < 70) return '#eab308' // yellow
  return '#22c55e' // green
})

const strengthText = computed(() => {
  if (strength.value < 40) return '弱'
  if (strength.value < 70) return '中'
  return '强'
})

// 复制
const copy = async (text: string) => {
  if (text) {
    await navigator.clipboard.writeText(text)
    message.success('已复制')
  }
}

// 监听配置变化自动生成
watch(
  [length, includeUppercase, includeLowercase, includeNumbers, includeSymbols, excludeSimilar],
  () => {
    generate()
  },
)

// 初始化生成一次
generate()
</script>

<template>
  <div class="h-full flex flex-col space-y-6">
    <div class="flex items-center justify-between pb-4 border-b border-slate-700/50">
      <div>
        <h2 class="text-2xl font-bold text-slate-100 tracking-tight">随机密码生成</h2>
        <p class="text-slate-500 text-sm mt-1">高强度、加密安全的随机密码</p>
      </div>
      <n-button quaternary size="small" @click="history = []">
        <template #icon><n-icon :component="Clean" /></template>
        清空历史
      </n-button>
    </div>

    <div class="bg-[#0F172A] p-6 rounded-2xl border border-slate-700 shadow-xl relative group">
      <div class="flex items-center space-x-4">
        <div
          class="flex-1 font-mono text-3xl tracking-wider text-cyan-400 break-all text-center select-all cursor-text"
        >
          {{ password }}
        </div>

        <div class="flex flex-col space-y-2">
          <n-button
            circle
            type="primary"
            size="large"
            @click="copy(password)"
            class="shadow-lg shadow-cyan-500/20"
          >
            <template #icon><n-icon :component="Copy" /></template>
          </n-button>
          <n-button circle secondary @click="generate">
            <template #icon><n-icon :component="Renew" /></template>
          </n-button>
        </div>
      </div>

      <div class="mt-6 flex items-center space-x-3">
        <div class="text-xs font-bold text-slate-500 w-8">{{ strengthText }}</div>
        <n-progress
          type="line"
          :percentage="strength"
          :color="strengthColor"
          :rail-color="'#1e293b'"
          :height="6"
          :show-indicator="false"
        />
      </div>
    </div>

    <div
      class="bg-[#0F172A]/40 p-5 rounded-xl border border-slate-700/50 grid grid-cols-1 md:grid-cols-2 gap-8"
    >
      <div class="space-y-4">
        <div class="flex justify-between items-center">
          <label class="text-slate-400 font-medium">密码长度</label>
          <n-input-number v-model:value="length" size="small" :min="4" :max="64" class="w-20" />
        </div>
        <n-slider v-model:value="length" :min="4" :max="64" :step="1" />

        <div class="pt-4">
          <label class="text-slate-400 font-medium block mb-3">选项开关</label>
          <div class="grid grid-cols-2 gap-3">
            <div
              class="flex items-center justify-between bg-slate-800/50 p-2 rounded border border-slate-700/50"
            >
              <span class="text-sm text-slate-300">A-Z 大写</span>
              <n-switch v-model:value="includeUppercase" size="small" />
            </div>
            <div
              class="flex items-center justify-between bg-slate-800/50 p-2 rounded border border-slate-700/50"
            >
              <span class="text-sm text-slate-300">a-z 小写</span>
              <n-switch v-model:value="includeLowercase" size="small" />
            </div>
            <div
              class="flex items-center justify-between bg-slate-800/50 p-2 rounded border border-slate-700/50"
            >
              <span class="text-sm text-slate-300">0-9 数字</span>
              <n-switch v-model:value="includeNumbers" size="small" />
            </div>
            <div
              class="flex items-center justify-between bg-slate-800/50 p-2 rounded border border-slate-700/50"
            >
              <span class="text-sm text-slate-300">#@% 符号</span>
              <n-switch v-model:value="includeSymbols" size="small" />
            </div>
          </div>

          <div
            class="mt-3 flex items-center justify-between bg-slate-800/30 p-2 rounded border border-slate-700/30"
          >
            <span class="text-sm text-slate-400">排除易混淆字符 (1 l I 0 O)</span>
            <n-switch v-model:value="excludeSimilar" size="small" />
          </div>
        </div>
      </div>

      <div class="flex flex-col h-full min-h-0">
        <label class="text-slate-400 font-medium mb-3">最近生成</label>
        <div class="flex-1 space-y-2 overflow-y-auto pr-1">
          <div
            v-for="pwd in history"
            :key="pwd"
            class="group flex items-center justify-between p-2 rounded hover:bg-slate-800/80 border border-transparent hover:border-slate-700 transition-colors cursor-pointer"
            @click="copy(pwd)"
          >
            <span class="font-mono text-slate-300 truncate mr-2 text-sm">{{ pwd }}</span>
            <n-icon :component="Copy" class="text-slate-500 opacity-0 group-hover:opacity-100" />
          </div>

          <div v-if="history.length === 0" class="text-slate-600 text-sm text-center py-4 italic">
            暂无历史
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
