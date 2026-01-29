<script setup lang="ts">
import { ref, computed } from 'vue'
import { NInput, NTag, NIcon, useMessage } from 'naive-ui'
import { CheckmarkFilled, ErrorFilled, Time, Code } from '@vicons/carbon'

const token = ref('')

// 核心：手动实现 Base64Url 解码 (支持 UTF-8)
const base64UrlDecode = (str: string) => {
  try {
    // 1. 补全 padding
    let output = str.replace(/-/g, '+').replace(/_/g, '/')
    switch (output.length % 4) {
      case 0: break
      case 2: output += '=='; break
      case 3: output += '='; break
      default: throw new Error('Illegal base64url string!')
    }
    // 2. 解码并处理 UTF-8 乱码问题
    return decodeURIComponent(atob(output).split('').map(c => {
      return '%' + ('00' + c.charCodeAt(0).toString(16)).slice(-2)
    }).join(''))
  } catch (e) {
    return null
  }
}

// 格式化时间戳
const formatTime = (timestamp: number) => {
  if (!timestamp) return ''
  // JWT 默认是秒，JS 需要毫秒
  return new Date(timestamp * 1000).toLocaleString()
}

// 解析逻辑
const parsedData = computed(() => {
  const parts = token.value.split('.')

  if (parts.length !== 3) {
    return { valid: false, header: null, payload: null, signature: null }
  }

  const headerStr = base64UrlDecode(parts[0])
  const payloadStr = base64UrlDecode(parts[1])

  if (!headerStr || !payloadStr) {
    return { valid: false, header: null, payload: null, signature: null }
  }

  try {
    return {
      valid: true,
      header: JSON.parse(headerStr),
      payload: JSON.parse(payloadStr),
      signature: parts[2]
    }
  } catch (e) {
    return { valid: false, header: null, payload: null, signature: null }
  }
})

// 判断 Token 是否过期
const isExpired = computed(() => {
  if (!parsedData.value.payload?.exp) return false
  const now = Math.floor(Date.now() / 1000)
  return parsedData.value.payload.exp < now
})
</script>

<template>
  <div class="h-full flex flex-col p-6 max-w-6xl mx-auto space-y-6">
    <div class="flex items-center justify-between">
      <div>
        <h2 class="text-2xl font-bold text-slate-100 flex items-center gap-2">
          <n-icon :component="Code" class="text-purple-400"/> JWT 解析器
        </h2>
        <p class="text-slate-500 text-sm mt-1">纯本地解析 · 隐私安全 · 自动时间格式化</p>
      </div>
      <div v-if="token">
        <n-tag v-if="parsedData.valid" :type="isExpired ? 'error' : 'success'" round size="large">
          <template #icon><n-icon :component="isExpired ? Time : CheckmarkFilled" /></template>
          {{ isExpired ? 'Token 已过期' : '格式有效' }}
        </n-tag>
        <n-tag v-else type="error" round size="large">
          <template #icon><n-icon :component="ErrorFilled" /></template>
          无效的 Token
        </n-tag>
      </div>
    </div>

    <div class="bg-[#1E293B] p-4 rounded-xl border border-slate-700 shadow-sm">
      <n-input
        v-model:value="token"
        type="textarea"
        placeholder="粘贴 JWT String (e.g. eyJhbGciOiJIUzI1NiIsInR5c...)"
        :autosize="{ minRows: 3, maxRows: 6 }"
        class="font-mono text-sm"
      />
    </div>

    <div v-if="parsedData.valid" class="grid grid-cols-1 md:grid-cols-2 gap-6 flex-1 min-h-0 overflow-hidden">

      <div class="flex flex-col gap-2">
        <div class="text-xs font-bold text-red-400 uppercase tracking-wider">Header</div>
        <div class="bg-[#0F172A] border-l-4 border-red-500 rounded-r-lg p-4 font-mono text-sm text-slate-300 overflow-auto h-full shadow-inner">
          <pre>{{ JSON.stringify(parsedData.header, null, 2) }}</pre>
        </div>
      </div>

      <div class="flex flex-col gap-2">
        <div class="text-xs font-bold text-purple-400 uppercase tracking-wider">Payload</div>
        <div class="bg-[#0F172A] border-l-4 border-purple-500 rounded-r-lg p-4 font-mono text-sm text-slate-300 overflow-auto h-full shadow-inner relative">
          <div class="space-y-1">
            <div v-for="(val, key) in parsedData.payload" :key="key" class="hover:bg-white/5 px-1 -mx-1 rounded">
              <span class="text-purple-300">"{{ key }}":</span>

              <span v-if="['exp', 'iat', 'nbf'].includes(String(key))" class="text-orange-300">
                 {{ val }} <span class="text-slate-500 text-xs ml-2 select-none">// {{ formatTime(val as number) }}</span>
               </span>

              <span v-else class="text-green-300"> {{ JSON.stringify(val) }}</span>,
            </div>
          </div>
        </div>
      </div>
    </div>

    <div v-else-if="!token" class="flex-1 flex flex-col items-center justify-center text-slate-600 border-2 border-dashed border-slate-800 rounded-2xl">
      <n-icon :size="64" :component="Code" class="mb-4 opacity-50"/>
      <p>在此粘贴 Token 开始解析</p>
    </div>
  </div>
</template>