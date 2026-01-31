<script setup lang="ts">
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'
import {
  NButton,
  NIcon,
  NCard,
  NList,
  NListItem,
  useMessage,
  NInput,
  NSelect,
  NSpin,
} from 'naive-ui'
import { Package, Deploy, DocumentAdd, FolderAdd, Close, FolderOpen } from '@vicons/carbon'

const message = useMessage()

// ========== 状态 ==========
const loading = ref(false)
const loadingText = ref('')

// ========== Pack 数据 ==========
const selectedFiles = ref<string[]>([])
const packPassword = ref('')
const compressionLevel = ref<number>(9)

const levelOptions = [
  { label: '最高压缩', value: 9 },
  { label: '平衡', value: 6 },
  { label: '最快', value: 1 },
]

// ========== Unpack 数据 ==========
const archivePath = ref('')
const extractDir = ref('')
const unpackPassword = ref('')

// ========== 辅助函数 ==========
const dirname = (p: string) => {
  // 简单的跨平台获取目录名
  const i = Math.max(p.lastIndexOf('/'), p.lastIndexOf('\\'))
  return i >= 0 ? p.slice(0, i) : ''
}

const addFiles = async () => {
  const selected = await open({ multiple: true, directory: false })
  if (!selected) return
  const list = Array.isArray(selected) ? selected : [selected]
  selectedFiles.value = [...new Set([...selectedFiles.value, ...list])]
}

const addFolder = async () => {
  const selected = await open({ multiple: true, directory: true })
  if (!selected) return
  const list = Array.isArray(selected) ? selected : [selected]
  selectedFiles.value = [...new Set([...selectedFiles.value, ...list])]
}

const removeFile = (index: number) => selectedFiles.value.splice(index, 1)

const canPack = computed(() => selectedFiles.value.length > 0)
const canUnpack = computed(() => !!archivePath.value && !!extractDir.value)

// ========== Pack 逻辑 ==========
const handlePack = async () => {
  if (!selectedFiles.value.length) return

  try {
    const savePath = await save({
      defaultPath: 'data.krate',
      filters: [{ name: 'Krate Package', extensions: ['krate'] }],
    })
    if (!savePath) return

    loading.value = true
    loadingText.value = '正在归档中，请稍候...'

    // 调用后端，await 会一直等待直到任务结束
    await invoke('create_archive', {
      inputs: selectedFiles.value,
      outputPath: savePath,
      password: packPassword.value.trim() || null,
      gzipLevel: compressionLevel.value,
    })

    message.success('打包成功！已生成 .krate 文件')
    // 清理状态
    selectedFiles.value = []
    packPassword.value = ''
  } catch (e: any) {
    message.error('打包失败: ' + (e?.message || e))
  } finally {
    loading.value = false
  }
}

// ========== Unpack 逻辑 ==========
const selectArchive = async () => {
  const selected = await open({ filters: [{ name: 'Krate Package', extensions: ['krate'] }] })
  if (selected && typeof selected === 'string') {
    archivePath.value = selected
    extractDir.value = dirname(selected)
  }
}

const selectExtractDir = async () => {
  const selected = await open({ directory: true })
  if (selected && typeof selected === 'string') extractDir.value = selected
}

const handleUnpack = async () => {
  if (!archivePath.value || !extractDir.value) return

  try {
    loading.value = true
    loadingText.value = '正在验证密钥并解压...'

    await invoke('extract_archive', {
      archivePath: archivePath.value,
      outputDir: extractDir.value,
      password: unpackPassword.value.trim() || null,
    })

    message.success('解压成功！')
  } catch (e: any) {
    message.error('解压失败: ' + (e?.message || e))
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="h-full flex flex-col p-6 space-y-4 relative">
    <div
      v-if="loading"
      class="absolute inset-0 z-50 bg-slate-900/80 backdrop-blur-sm flex flex-col items-center justify-center rounded-xl select-none"
    >
      <div
        class="bg-slate-800 p-8 rounded-2xl border border-slate-700 shadow-2xl flex flex-col items-center"
      >
        <NSpin size="large" />
        <div class="mt-4 text-emerald-400 font-bold text-lg animate-pulse">{{ loadingText }}</div>
        <div class="mt-2 text-xs text-slate-500">大文件处理可能需要一些时间</div>
      </div>
    </div>

    <div class="flex justify-between items-center">
      <h2 class="text-xl font-bold text-slate-100 flex items-center gap-2">
        <n-icon class="text-emerald-400"><Package /></n-icon>
        Krate 私有归档
      </h2>
    </div>

    <div class="flex-1 grid grid-cols-2 gap-6 min-h-0">
      <NCard
        title="创建归档"
        class="bg-slate-800/50 border-slate-700 flex flex-col h-full"
        content-style="display: flex; flex-direction: column; height: 100%;"
      >
        <div class="flex gap-2 mb-4">
          <NButton dashed class="flex-1" @click="addFiles">
            <template #icon
              ><NIcon><DocumentAdd /></NIcon
            ></template>
            添加文件
          </NButton>
          <NButton dashed class="flex-1" @click="addFolder">
            <template #icon
              ><NIcon><FolderAdd /></NIcon
            ></template>
            添加文件夹
          </NButton>
        </div>

        <div class="flex gap-3 mb-3">
          <div class="flex-1">
            <div class="text-xs text-slate-400 mb-1">压缩等级</div>
            <NSelect v-model:value="compressionLevel" :options="levelOptions" size="small" />
          </div>
          <div class="flex-2">
            <div class="text-xs text-slate-400 mb-1">密码（可选）</div>
            <NInput
              v-model:value="packPassword"
              size="small"
              type="password"
              show-password-on="click"
              placeholder="留空则不加密"
            />
          </div>
        </div>

        <div
          class="flex-1 overflow-y-auto bg-slate-900/30 rounded border border-slate-700/50 p-2 custom-scrollbar"
        >
          <NList v-if="selectedFiles.length > 0">
            <NListItem v-for="(file, index) in selectedFiles" :key="file">
              <div class="flex justify-between items-center group">
                <span class="text-xs text-slate-300 truncate mr-2 font-mono">{{ file }}</span>
                <NButton
                  size="tiny"
                  quaternary
                  type="error"
                  @click="removeFile(index)"
                  class="opacity-0 group-hover:opacity-100"
                >
                  <template #icon
                    ><NIcon><Close /></NIcon
                  ></template>
                </NButton>
              </div>
            </NListItem>
          </NList>
          <div v-else class="h-full flex items-center justify-center text-slate-600 text-sm">
            请添加文件或文件夹
          </div>
        </div>

        <div class="mt-4">
          <NButton type="primary" block @click="handlePack" :disabled="!canPack || loading">
            生成 .krate 包
          </NButton>
        </div>
      </NCard>

      <NCard
        title="还原归档"
        class="bg-slate-800/50 border-slate-700"
        content-style="display: flex; flex-direction: column; height: 100%;"
      >
        <div class="flex-1 flex flex-col justify-center space-y-6 px-4">
          <div
            class="border-2 border-dashed border-slate-700 rounded-xl h-32 flex flex-col items-center justify-center cursor-pointer hover:border-emerald-500/50 transition-colors"
            @click="selectArchive"
          >
            <div v-if="!archivePath" class="text-center text-slate-500">
              <NIcon size="40" class="mb-2"><FolderOpen /></NIcon>
              <div>点击选择 .krate 文件</div>
            </div>
            <div v-else class="text-center px-4">
              <NIcon size="32" class="text-emerald-500 mb-1"><Package /></NIcon>
              <div class="text-xs text-slate-300 break-all">{{ archivePath }}</div>
            </div>
          </div>

          <div class="space-y-2">
            <div class="text-xs text-slate-400">解压位置</div>
            <NInput
              size="small"
              v-model:value="extractDir"
              placeholder="默认解压到同级目录"
              readonly
              @click="selectExtractDir"
            >
              <template #suffix
                ><NIcon><FolderOpen /></NIcon
              ></template>
            </NInput>
          </div>

          <div class="space-y-2">
            <div class="text-xs text-slate-400">解压密码</div>
            <NInput
              v-model:value="unpackPassword"
              size="small"
              type="password"
              show-password-on="click"
              placeholder="若文件已加密则必填"
            />
          </div>
        </div>

        <div class="mt-4">
          <NButton
            type="success"
            secondary
            block
            @click="handleUnpack"
            :disabled="!canUnpack || loading"
          >
            <template #icon
              ><NIcon><Deploy /></NIcon
            ></template>
            开始解压
          </NButton>
        </div>
      </NCard>
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
