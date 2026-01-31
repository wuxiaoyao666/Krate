<script setup lang="ts">
import { useMessage, NList, NListItem, NButton, NIcon, NTag, NSwitch } from 'naive-ui'
import { Copy, TrashCan, Pause, Play, Clean } from '@vicons/carbon'
import { HistoryItem, useClipboard } from '@/hooks/useClipboard' // 引入全局状态

const message = useMessage()
// 直接解构获取全局状态和方法
const { history, isListening, copyItem, deleteItem, clearHistory } = useClipboard()

const handleCopy = async (item: HistoryItem) => {
  const success = await copyItem(item)
  if (success) {
    message.success('已复制')
  } else {
    message.error('复制失败')
  }
}

const handleClear = () => {
  clearHistory()
  message.success('记录已清空')
}

const formatTime = (ts: number) => {
  return new Date(ts).toLocaleString('zh-CN', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  })
}
</script>

<template>
  <div class="h-full flex flex-col p-6 space-y-4">
    <div class="flex items-center justify-between pb-4 border-b border-slate-700/50">
      <div>
        <h2 class="text-2xl font-bold text-slate-100 flex items-center gap-2">
          <NIcon class="text-emerald-400"><Copy /></NIcon>
          剪切板历史
        </h2>
        <p class="text-slate-400 text-sm mt-1">全局后台监听 · 自动去重</p>
      </div>

      <div class="flex items-center gap-4">
        <div
          class="flex items-center gap-2 bg-slate-800/50 px-3 py-1.5 rounded-full border border-slate-700"
        >
          <span class="text-xs text-slate-400">
            {{ isListening ? '监听中' : '已暂停' }}
          </span>
          <NSwitch v-model:value="isListening" size="small">
            <template #checked-icon
              ><NIcon><Play /></NIcon
            ></template>
            <template #unchecked-icon
              ><NIcon><Pause /></NIcon
            ></template>
          </NSwitch>
        </div>

        <NButton
          size="small"
          quaternary
          type="error"
          @click="handleClear"
          :disabled="history.length === 0"
        >
          <template #icon
            ><NIcon><Clean /></NIcon
          ></template>
          清空
        </NButton>
      </div>
    </div>

    <div class="flex-1 overflow-y-auto pr-2 custom-scrollbar">
      <NList v-if="history.length > 0" hoverable clickable class="bg-transparent">
        <NListItem
          v-for="(item, index) in history"
          :key="item.id"
          class="mb-3 bg-slate-800/40 rounded-xl border border-slate-700/50 hover:border-emerald-500/30 transition-all group"
        >
          <div class="flex flex-col gap-2 p-1">
            <div class="flex justify-between items-center">
              <NTag
                size="small"
                :bordered="false"
                class="bg-slate-700/50 text-slate-400 text-[10px] h-5"
              >
                {{ formatTime(item.time) }}
              </NTag>
              <div class="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                <NButton size="tiny" secondary type="primary" @click.stop="handleCopy(item)">
                  复制
                </NButton>
                <NButton size="tiny" quaternary type="error" @click.stop="deleteItem(index)">
                  <template #icon
                    ><NIcon><TrashCan /></NIcon
                  ></template>
                </NButton>
              </div>
            </div>

            <div
              class="text-sm text-slate-200 font-mono break-all whitespace-pre-wrap max-h-24 overflow-hidden relative cursor-pointer"
              @click.stop="handleCopy(item)"
              title="点击复制"
            >
              {{ item.text }}
              <div
                class="absolute bottom-0 left-0 right-0 h-6 bg-linear-to-t from-[#1e293b] to-transparent pointer-events-none"
              />
            </div>
          </div>
        </NListItem>
      </NList>

      <div v-else class="h-full flex flex-col items-center justify-center text-slate-600 space-y-4">
        <NIcon size="64" class="opacity-20"><Copy /></NIcon>
        <div class="text-sm">
          {{ isListening ? '暂无记录，试着复制一段文字看看' : '监听已暂停，打开开关开始记录' }}
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.1);
  border-radius: 2px;
}
</style>
