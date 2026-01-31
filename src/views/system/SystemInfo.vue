<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { NCard, NGrid, NGridItem, NIcon, NProgress, NSkeleton, NStatistic, NTag } from 'naive-ui'
// 修复 4: 替换 Desktop 为 Screen (因为 @vicons/carbon 没有 Desktop)
import { Chip, DataCenter, DataRefinery, Screen, Time } from '@vicons/carbon'

interface SystemInfo {
  cpuBrand: string
  cpuUsage: number
  cpuCores: number // 物理核心
  cpuLogicalCores: number // 逻辑核心
  totalMemory: number
  usedMemory: number
  totalSwap: number
  usedSwap: number
  osName: string
  osVersion: string
  hostName: string
  kernelVersion: string
  uptime: number
}

const info = ref<SystemInfo | null>(null)
const timer = ref<number | null>(null)

// 工具：格式化字节
const formatBytes = (bytes: number) => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

// 工具：格式化时间
const formatUptime = (seconds: number) => {
  const d = Math.floor(seconds / (3600 * 24))
  const h = Math.floor((seconds % (3600 * 24)) / 3600)
  const m = Math.floor((seconds % 3600) / 60)

  let res = ''
  if (d > 0) res += `${d}天 `
  if (h > 0) res += `${h}小时 `
  res += `${m}分钟`
  return res
}

const fetchInfo = async () => {
  try {
    info.value = await invoke<SystemInfo>('get_system_info')
  } catch (e) {
    console.error('Failed to fetch system info:', e)
  }
}

onMounted(() => {
  fetchInfo()
  timer.value = window.setInterval(fetchInfo, 2000)
})

onUnmounted(() => {
  if (timer.value) clearInterval(timer.value)
})
</script>

<template>
  <div class="h-full flex flex-col p-6 space-y-6 overflow-y-auto">
    <div class="flex items-center justify-between">
      <div>
        <h2 class="text-2xl font-bold text-slate-100 flex items-center gap-2">
          <NIcon class="text-emerald-400"><Screen /></NIcon>
          系统监控
        </h2>
        <div v-if="info" class="text-slate-400 text-sm mt-1 flex gap-2 items-center">
          <span>{{ info.osName }} {{ info.osVersion }}</span>
          <span class="w-1 h-1 bg-slate-600 rounded-full"></span>
          <span>{{ info.kernelVersion }}</span>
          <span class="w-1 h-1 bg-slate-600 rounded-full"></span>
          <span>{{ info.hostName }}</span>
        </div>
        <NSkeleton v-else text style="width: 200px" class="mt-2" />
      </div>

      <NTag
        type="primary"
        round
        v-if="info"
        :bordered="false"
        class="bg-slate-800 text-emerald-400"
      >
        <template #icon
          ><NIcon><Time /></NIcon
        ></template>
        运行时间: {{ formatUptime(info.uptime) }}
      </NTag>
    </div>

    <div v-if="!info" class="grid grid-cols-4 gap-4">
      <NSkeleton v-for="i in 4" :key="i" height="150px" class="rounded-xl bg-slate-800/50" />
    </div>

    <template v-else>
      <NGrid x-gap="16" y-gap="16" cols="1 s:2 m:2 l:4" responsive="screen">
        <NGridItem>
          <NCard
            class="bg-slate-800/50 border-slate-700 h-full hover:border-emerald-500/30 transition-colors"
          >
            <NStatistic label="CPU 使用率">
              <template #prefix>
                <NIcon class="text-blue-400"><Chip /></NIcon>
              </template>
              {{ info.cpuUsage.toFixed(1) }}
              <template #suffix><span class="text-sm text-slate-500">%</span></template>
            </NStatistic>
            <div class="mt-4">
              <NProgress
                type="line"
                :percentage="info.cpuUsage"
                :color="info.cpuUsage > 80 ? '#f87171' : '#60a5fa'"
                :show-indicator="false"
                processing
              />
            </div>
            <div
              class="mt-3 text-xs text-slate-400 truncate flex items-center gap-1"
              :title="info.cpuBrand"
            >
              <NIcon><Chip /></NIcon>
              {{ info.cpuBrand }}
            </div>
          </NCard>
        </NGridItem>

        <NGridItem>
          <NCard
            class="bg-slate-800/50 border-slate-700 h-full hover:border-emerald-500/30 transition-colors"
          >
            <NStatistic label="内存使用">
              <template #prefix>
                <NIcon class="text-emerald-400"><DataRefinery /></NIcon>
              </template>
              {{ ((info.usedMemory / info.totalMemory) * 100).toFixed(1) }}
              <template #suffix><span class="text-sm text-slate-500">%</span></template>
            </NStatistic>
            <div class="mt-4">
              <NProgress
                type="line"
                :percentage="(info.usedMemory / info.totalMemory) * 100"
                color="#34d399"
                :show-indicator="false"
              />
            </div>
            <div class="mt-3 text-xs text-slate-400 flex justify-between">
              <span>{{ formatBytes(info.usedMemory) }}</span>
              <span class="text-slate-600">/ {{ formatBytes(info.totalMemory) }}</span>
            </div>
          </NCard>
        </NGridItem>

        <NGridItem>
          <NCard
            class="bg-slate-800/50 border-slate-700 h-full flex flex-col justify-center hover:border-emerald-500/30 transition-colors"
          >
            <div class="flex flex-col h-full justify-between">
              <NStatistic label="核心数 (物理/逻辑)">
                <template #prefix>
                  <NIcon class="text-purple-400"><DataCenter /></NIcon>
                </template>
                {{ info.cpuCores }}
                <span class="text-slate-500 text-lg">/ {{ info.cpuLogicalCores }}</span>
              </NStatistic>
              <div class="mt-2 text-xs text-slate-500">高性能并行计算能力</div>
            </div>
          </NCard>
        </NGridItem>

        <NGridItem>
          <NCard
            class="bg-slate-800/50 border-slate-700 h-full hover:border-emerald-500/30 transition-colors"
          >
            <NStatistic label="交换空间 (Swap)">
              <template #prefix><span class="text-yellow-500">⇄</span></template>
              {{ info.totalSwap > 0 ? ((info.usedSwap / info.totalSwap) * 100).toFixed(1) : 0 }}
              <template #suffix><span class="text-sm text-slate-500">%</span></template>
            </NStatistic>
            <div class="mt-4">
              <NProgress
                type="line"
                :percentage="info.totalSwap > 0 ? (info.usedSwap / info.totalSwap) * 100 : 0"
                color="#fbbf24"
                :show-indicator="false"
              />
            </div>
            <div class="mt-3 text-xs text-slate-400 flex justify-between">
              <span>{{ formatBytes(info.usedSwap) }}</span>
              <span class="text-slate-600">/ {{ formatBytes(info.totalSwap) }}</span>
            </div>
          </NCard>
        </NGridItem>
      </NGrid>

      <NCard title="环境详情" class="bg-slate-800/50 border-slate-700 mt-4">
        <div class="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
          <div>
            <div class="text-slate-500 mb-1">主机名</div>
            <div class="text-slate-200 font-mono">{{ info.hostName }}</div>
          </div>
          <div>
            <div class="text-slate-500 mb-1">操作系统</div>
            <div class="text-slate-200">{{ info.osName }} {{ info.osVersion }}</div>
          </div>
          <div>
            <div class="text-slate-500 mb-1">内核版本</div>
            <div class="text-slate-200 font-mono">{{ info.kernelVersion }}</div>
          </div>
          <div>
            <div class="text-slate-500 mb-1">系统架构</div>
            <div class="text-slate-200 font-mono">64-bit</div>
          </div>
        </div>
      </NCard>
    </template>
  </div>
</template>
