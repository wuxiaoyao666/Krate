<script setup lang="ts">
import { ref } from 'vue'
import { useMessage, NButton, NIcon, NTag, NSwitch, NPopconfirm } from 'naive-ui'
import { Copy, TrashCan, Pause, Play, Clean } from '@vicons/carbon'
import { HistoryItem, useClipboard } from '@/hooks/useClipboard'

const message = useMessage()
const { history, isListening, copyItem, deleteItem, clearHistory } = useClipboard()

const expandedIds = ref<Set<string>>(new Set())

const isLongText = (text: string) => text.length > 180 || text.split('\n').length > 4

const isExpanded = (id: string) => expandedIds.value.has(id)

const toggleExpand = (id: string) => {
  const next = new Set(expandedIds.value)
  if (next.has(id)) {
    next.delete(id)
  } else {
    next.add(id)
  }
  expandedIds.value = next
}

const clearExpanded = (id: string) => {
  const next = new Set(expandedIds.value)
  next.delete(id)
  expandedIds.value = next
}

const handleCopy = async (item: HistoryItem) => {
  const success = await copyItem(item)
  if (success) {
    message.success('已复制')
  } else {
    message.error('复制失败')
  }
}

const handleDelete = (index: number, id: string) => {
  deleteItem(index)
  clearExpanded(id)
}

const handleClear = () => {
  if (history.value.length === 0) return
  clearHistory()
  expandedIds.value = new Set()
  message.success('记录已清空')
}

const handleBodyClick = async (item: HistoryItem) => {
  if (isLongText(item.text)) {
    toggleExpand(item.id)
    return
  }
  await handleCopy(item)
}

const bodyTitle = (item: HistoryItem) => {
  if (isLongText(item.text)) {
    return isExpanded(item.id) ? '点击收起' : '点击展开'
  }
  return '点击复制'
}

const formatTime = (ts: number) => {
  const date = new Date(ts)
  const now = new Date()
  const timePart = date.toLocaleTimeString('zh-CN', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  })

  if (date.toDateString() === now.toDateString()) {
    return `今天 ${timePart}`
  }

  if (date.getFullYear() === now.getFullYear()) {
    const dayPart = date.toLocaleDateString('zh-CN', {
      month: '2-digit',
      day: '2-digit',
    })
    return `${dayPart} ${timePart}`
  }

  const fullDatePart = date.toLocaleDateString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  })
  return `${fullDatePart} ${timePart}`
}
</script>

<template>
  <div class="h-full flex flex-col p-6 gap-4">
    <div class="flex items-center justify-between pb-4 border-b border-slate-700/50">
      <div>
        <h2 class="text-2xl font-bold text-slate-100 flex items-center gap-2">
          <NIcon class="text-emerald-400"><Copy /></NIcon>
          剪贴板历史
        </h2>
        <p class="text-slate-400 text-sm mt-1">全局后台监听 · 自动去重</p>
      </div>

      <div class="flex items-center gap-3">
        <div class="text-xs text-slate-500 tracking-wide">共 {{ history.length }} 条记录</div>

        <div
          class="flex items-center gap-2 bg-slate-800/60 px-3 py-1.5 rounded-full border border-slate-700"
        >
          <span class="text-xs text-slate-400">{{ isListening ? '监听中' : '已暂停' }}</span>
          <NSwitch v-model:value="isListening" size="small">
            <template #checked-icon><NIcon><Play /></NIcon></template>
            <template #unchecked-icon><NIcon><Pause /></NIcon></template>
          </NSwitch>
        </div>

        <NPopconfirm positive-text="清空" negative-text="取消" @positive-click="handleClear">
          <template #trigger>
            <NButton size="small" quaternary type="error" :disabled="history.length === 0">
              <template #icon><NIcon><Clean /></NIcon></template>
              清空
            </NButton>
          </template>
          确认清空全部剪贴板历史吗？
        </NPopconfirm>
      </div>
    </div>

    <div class="flex-1 overflow-y-auto pr-2 custom-scrollbar">
      <div v-if="history.length > 0" class="space-y-3 pb-1">
        <article
          v-for="(item, index) in history"
          :key="item.id"
          class="rounded-2xl border border-slate-700/70 bg-linear-to-br from-slate-800/70 via-slate-800/45 to-slate-900/70 p-4 shadow-sm shadow-black/10 hover:border-emerald-500/35 hover:shadow-emerald-500/10 transition-all duration-200"
        >
          <div class="flex items-start justify-between gap-3">
            <div class="flex items-center gap-2 min-w-0">
              <NTag size="small" :bordered="false" class="bg-slate-700/60 text-slate-300">
                {{ formatTime(item.time) }}
              </NTag>
              <span class="text-[11px] text-slate-500 shrink-0">{{ item.text.length }} 字符</span>
            </div>

            <div class="flex items-center gap-1.5 shrink-0">
              <NButton size="tiny" secondary type="primary" @click.stop="handleCopy(item)">
                复制
              </NButton>
              <NButton
                v-if="isLongText(item.text)"
                size="tiny"
                quaternary
                @click.stop="toggleExpand(item.id)"
              >
                {{ isExpanded(item.id) ? '收起' : '展开' }}
              </NButton>
              <NButton size="tiny" quaternary type="error" @click.stop="handleDelete(index, item.id)">
                <template #icon><NIcon><TrashCan /></NIcon></template>
              </NButton>
            </div>
          </div>

          <div
            class="mt-3 text-sm text-slate-100 font-mono break-words whitespace-pre-wrap leading-relaxed rounded-xl border border-slate-700/40 px-3 py-2 bg-slate-900/25 cursor-pointer relative"
            :class="{ 'max-h-24 overflow-hidden': !isExpanded(item.id) && isLongText(item.text) }"
            @click.stop="handleBodyClick(item)"
            :title="bodyTitle(item)"
          >
            {{ item.text }}
            <div
              v-if="!isExpanded(item.id) && isLongText(item.text)"
              class="absolute bottom-0 left-0 right-0 h-8 bg-linear-to-t from-[#0f172a] to-transparent pointer-events-none"
            />
          </div>
        </article>
      </div>

      <div v-else class="h-full flex flex-col items-center justify-center text-slate-600 space-y-4">
        <NIcon size="64" class="opacity-20"><Copy /></NIcon>
        <div class="text-sm">
          {{
            isListening
              ? '暂无记录，试着复制一段文字看看'
              : '监听已暂停，打开开关后开始记录剪贴板历史'
          }}
        </div>
      </div>
    </div>
  </div>
</template>
