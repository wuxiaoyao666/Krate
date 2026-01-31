<script setup lang="ts">
import { ref, watch } from 'vue'
import CryptoJS from 'crypto-js'
import { NInput, NSelect, NButton, useMessage } from 'naive-ui'
import { Copy, Clean } from '@vicons/carbon'
import { NIcon } from 'naive-ui'
import bcrypt from 'bcryptjs'

const message = useMessage()

// 状态定义
const content = ref('')
const salt = ref('')
const algorithm = ref('MD5')
const result = ref('')
const isUpperCase = ref(false)

// 算法选项
const options = [
  { label: 'MD5', value: 'MD5' },
  { label: 'SHA1', value: 'SHA1' },
  { label: 'SHA256', value: 'SHA256' },
  { label: 'SHA512', value: 'SHA512' },
  { label: 'HmacMD5', value: 'HmacMD5' },
  { label: 'HmacSHA256', value: 'HmacSHA256' },
  { label: 'Bcrypt (密码专用)', value: 'Bcrypt' },
]

// 核心计算逻辑
const calculate = () => {
  if (!content.value) {
    result.value = ''
    return
  }

  let output = ''
  try {
    if (algorithm.value === 'Bcrypt') {
      output = bcrypt.hashSync(content.value, 10)
    } else {
      const textToHash = content.value + salt.value // 简单的加盐逻辑
      let hashObj

      // 根据选择调用 crypto-js 对应方法
      switch (algorithm.value) {
        case 'MD5':
          hashObj = CryptoJS.MD5(textToHash)
          break
        case 'SHA1':
          hashObj = CryptoJS.SHA1(textToHash)
          break
        case 'SHA256':
          hashObj = CryptoJS.SHA256(textToHash)
          break
        case 'SHA512':
          hashObj = CryptoJS.SHA512(textToHash)
          break
        case 'HmacMD5':
          hashObj = CryptoJS.HmacMD5(content.value, salt.value)
          break // Hmac 用法不同
        case 'HmacSHA256':
          hashObj = CryptoJS.HmacSHA256(content.value, salt.value)
          break
        default:
          hashObj = CryptoJS.MD5(textToHash)
      }
      output = hashObj.toString()
    }

    // 格式化输出
    if (isUpperCase.value) {
      output = output.toUpperCase()
    }
    result.value = output
  } catch (error) {
    console.error(error)
    result.value = '计算错误'
  }
}

// 监听变化，实时计算
watch([content, salt, algorithm, isUpperCase], () => {
  calculate()
})

// 复制功能
const copyResult = async () => {
  if (result.value) {
    await navigator.clipboard.writeText(result.value)
    message.success('已复制到剪贴板')
  }
}

// 清空
const clearAll = () => {
  content.value = ''
  salt.value = ''
  result.value = ''
}
</script>

<template>
  <div class="h-full flex flex-col space-y-6">
    <div class="flex items-center justify-between pb-4 border-b border-slate-700/50">
      <div>
        <h2 class="text-2xl font-bold text-slate-100 tracking-tight">哈希计算</h2>
      </div>
      <n-button quaternary size="small" @click="clearAll">
        <template #icon><n-icon :component="Clean" /></template>
        清空
      </n-button>
    </div>

    <div
      class="bg-[#0F172A]/40 p-5 rounded-xl border border-slate-700/50 grid grid-cols-1 md:grid-cols-12 gap-4"
    >
      <div class="md:col-span-4">
        <div class="text-xs text-slate-400 mb-2 font-medium">加密算法</div>
        <n-select v-model:value="algorithm" :options="options" />
      </div>

      <div class="md:col-span-6">
        <div class="text-xs text-slate-400 mb-2 font-medium">加盐 (Salt) / 密钥</div>
        <n-input v-model:value="salt" placeholder="可选：混淆字符串" clearable />
      </div>

      <div class="md:col-span-2 flex items-end">
        <div
          class="w-full h-[34px] flex items-center justify-center border border-slate-700 rounded cursor-pointer select-none transition-colors"
          :class="
            isUpperCase
              ? 'bg-cyan-500/20 border-cyan-500 text-cyan-400'
              : 'bg-slate-800 text-slate-400 hover:bg-slate-700'
          "
          @click="isUpperCase = !isUpperCase"
        >
          <span class="text-xs font-bold">{{ isUpperCase ? '大写' : '小写' }}</span>
        </div>
      </div>
    </div>

    <div class="flex-1 grid grid-cols-1 md:grid-cols-2 gap-6 min-h-0">
      <div class="flex flex-col h-full">
        <label class="text-sm font-medium text-slate-400 mb-2 pl-1">原始文本</label>
        <n-input
          v-model:value="content"
          type="textarea"
          placeholder="在此输入需要计算的内容..."
          class="flex-1 font-mono text-base !bg-slate-900/50 border-slate-700 hover:border-cyan-500/50 focus:border-cyan-500 rounded-xl p-2"
          :input-props="{ style: 'height: 100%;' }"
        />
      </div>

      <div class="flex flex-col h-full relative group">
        <label class="text-sm font-medium text-slate-400 mb-2 pl-1">计算结果</label>
        <div class="flex-1 relative">
          <n-input
            v-model:value="result"
            type="textarea"
            readonly
            placeholder="结果将在此显示"
            class="h-full font-mono text-lg !bg-[#0F172A] border-cyan-900/30 text-cyan-400 rounded-xl p-2 shadow-inner"
            :input-props="{ style: 'height: 100%; resize: none;' }"
          />

          <div class="absolute bottom-4 right-4" v-if="result">
            <n-button
              type="primary"
              size="large"
              @click="copyResult"
              class="shadow-lg shadow-cyan-500/20"
            >
              <template #icon><n-icon :component="Copy" /></template>
              复制结果
            </n-button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 覆盖 Naive UI Input 默认样式，让其更贴合 Tailwind */
:deep(.n-input) {
  background-color: transparent;
}
:deep(.n-input__textarea-el) {
  height: 100% !important;
}
</style>
