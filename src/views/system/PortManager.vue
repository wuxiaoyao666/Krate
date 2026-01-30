<script setup lang="ts">
import { ref, onMounted, h } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { NButton, NDataTable, NTag, useMessage, NInput, NSpace } from 'naive-ui'
import type { DataTableColumns } from 'naive-ui'

interface PortInfo {
  pid: string
  port: string
  protocol: string
  program: string
}

const message = useMessage()
const loading = ref(false)
const portList = ref<PortInfo[]>([])
const searchText = ref('')

const columns: DataTableColumns<PortInfo> = [
  { title: 'PID', key: 'pid', width: 100, sorter: (a, b) => Number(a.pid) - Number(b.pid) },
  {
    title: '端口',
    key: 'port',
    width: 100,
    sorter: (a, b) => Number(a.port) - Number(b.port),
    render(row) {
      return h(NTag, { type: 'success', bordered: false }, { default: () => row.port })
    }
  },
  { title: '协议', key: 'protocol', width: 80 },
  { title: '程序/进程', key: 'program', width: 150 },
  {
    title: '操作',
    key: 'actions',
    width: 100,
    render(row) {
      return h(
        NButton,
        {
          size: 'small',
          type: 'error',
          secondary: true,
          onClick: () => handleKill(row.pid)
        },
        { default: () => '结束进程' }
      )
    }
  }
]

// 刷新列表
const refreshPorts = async () => {
  loading.value = true
  try {
    const res = await invoke<PortInfo[]>('scan_ports')
    // Windows 下 netstat 可能有重复行（监听不同 IP），可以简单去重
    const uniqueMap = new Map()
    res.forEach(item => {
      // 用 "端口-PID" 作为唯一键
      const key = `${item.port}-${item.pid}`
      if (!uniqueMap.has(key)) {
        uniqueMap.set(key, item)
      }
    })
    portList.value = Array.from(uniqueMap.values())
  } catch (error) {
    message.error('扫描失败: ' + error)
  } finally {
    loading.value = false
  }
}

// 结束进程
const handleKill = async (pid: string) => {
  try {
    await invoke('kill_process', { pid })
    message.success(`进程 ${pid} 已结束`)
    refreshPorts() // 重新扫描
  } catch (error) {
    message.error(`无法结束进程: ${error}`)
  }
}

// 过滤列表
const filteredData = computed(() => {
  if (!searchText.value) return portList.value
  const lower = searchText.value.toLowerCase()
  return portList.value.filter(item =>
    item.port.includes(lower) ||
    item.pid.includes(lower) ||
    item.program.toLowerCase().includes(lower)
  )
})

onMounted(() => {
  refreshPorts()
})

import { computed } from 'vue'
import { Renew } from '@vicons/carbon'
import { NIcon } from 'naive-ui'
</script>

<template>
  <div class="h-full flex flex-col p-6 space-y-4">
    <div class="flex justify-between items-center">
      <h2 class="text-xl font-bold text-slate-100">端口探针</h2>
      <NSpace>
        <NInput v-model:value="searchText" placeholder="搜索端口或 PID..." class="w-64" clearable />
        <NButton type="primary" @click="refreshPorts" :loading="loading">
          <template #icon>
            <NIcon><Renew /></NIcon>
          </template>
          刷新
        </NButton>
      </NSpace>
    </div>

    <div class="flex-1 overflow-hidden bg-slate-800/50 rounded-lg border border-slate-700">
      <NDataTable
        :columns="columns"
        :data="filteredData"
        :loading="loading"
        :max-height="600"
        :row-key="(row) => row.port + row.pid"
        virtual-scroll
        flex-height
        class="h-full"
      />
    </div>
  </div>
</template>