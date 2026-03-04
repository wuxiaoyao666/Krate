<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { open, save } from '@tauri-apps/plugin-dialog'
import { openPath } from '@tauri-apps/plugin-opener'
import {
  NButton,
  NCard,
  NIcon,
  NInput,
  NList,
  NListItem,
  NProgress,
  NSelect,
  useMessage,
} from 'naive-ui'
import { Close, Deploy, DocumentAdd, FolderAdd, FolderOpen, Package } from '@vicons/carbon'

interface ArchiveProgressPayload {
  operation: 'pack' | 'extract'
  stage: string
  message: string
  progress: number
  currentPath?: string | null
}

const message = useMessage()

const loading = ref(false)
const loadingText = ref('')
const progress = ref(0)
const progressStage = ref('')
const progressCurrentPath = ref('')
const activeOperation = ref<'pack' | 'extract' | null>(null)
const lastExtractedDir = ref('')

const selectedFiles = ref<string[]>([])
const packPassword = ref('')
const compressionLevel = ref<number>(6)

const archivePath = ref('')
const extractDir = ref('')
const unpackPassword = ref('')

const normalizedPackPassword = computed(() => packPassword.value.trim())
const normalizedUnpackPassword = computed(() => unpackPassword.value.trim())
const canPack = computed(() => selectedFiles.value.length > 0)
const canUnpack = computed(() => !!archivePath.value && !!extractDir.value)
const progressPercent = computed(() => Math.round(progress.value))

const levelOptions = [
  { label: '平衡（推荐）', value: 6 },
  { label: '最高压缩', value: 9 },
  { label: '最快速度', value: 1 },
]

let unlistenProgress: UnlistenFn | null = null

const resetProgressState = () => {
  progress.value = 0
  progressStage.value = ''
  progressCurrentPath.value = ''
  loadingText.value = ''
  activeOperation.value = null
}

const dirname = (targetPath: string) => {
  const index = Math.max(targetPath.lastIndexOf('/'), targetPath.lastIndexOf('\\'))
  return index >= 0 ? targetPath.slice(0, index) : ''
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

const openExtractedDir = async () => {
  if (!lastExtractedDir.value) return

  try {
    await openPath(lastExtractedDir.value)
  } catch (error: any) {
    message.error('打开输出目录失败: ' + (error?.message || error))
  }
}

const handlePack = async () => {
  if (!selectedFiles.value.length) return

  try {
    const savePath = await save({
      defaultPath: 'data.krate',
      filters: [{ name: 'Krate Package', extensions: ['krate'] }],
    })
    if (!savePath) return

    loading.value = true
    activeOperation.value = 'pack'
    progress.value = 0
    progressStage.value = '准备归档'
    progressCurrentPath.value = ''
    loadingText.value = normalizedPackPassword.value ? '正在压缩并加密' : '正在压缩打包'

    await invoke('create_archive', {
      inputs: selectedFiles.value,
      outputPath: savePath,
      password: normalizedPackPassword.value || null,
      gzipLevel: compressionLevel.value,
    })

    message.success('打包成功，已生成 .krate 文件')
    selectedFiles.value = []
    packPassword.value = ''
  } catch (error: any) {
    message.error('打包失败: ' + (error?.message || error))
  } finally {
    loading.value = false
    resetProgressState()
  }
}

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
    activeOperation.value = 'extract'
    progress.value = 0
    progressStage.value = '读取归档头'
    progressCurrentPath.value = ''
    loadingText.value = normalizedUnpackPassword.value ? '正在校验密码并解压' : '正在解压归档'

    await invoke('extract_archive', {
      archivePath: archivePath.value,
      outputDir: extractDir.value,
      password: normalizedUnpackPassword.value || null,
    })

    lastExtractedDir.value = extractDir.value
    message.success('解压成功')
  } catch (error: any) {
    message.error('解压失败: ' + (error?.message || error))
  } finally {
    loading.value = false
    resetProgressState()
  }
}

onMounted(async () => {
  unlistenProgress = await listen<ArchiveProgressPayload>('archive://progress', (event) => {
    if (!loading.value || !activeOperation.value) return
    if (event.payload.operation !== activeOperation.value) return

    progress.value = Math.max(0, Math.min(100, event.payload.progress ?? 0))
    progressStage.value = event.payload.stage || progressStage.value
    loadingText.value = event.payload.message || loadingText.value
    progressCurrentPath.value = event.payload.currentPath || ''
  })
})

onBeforeUnmount(() => {
  if (unlistenProgress) {
    unlistenProgress()
    unlistenProgress = null
  }
})
</script>

<template>
  <div class="relative flex h-full flex-col space-y-4 p-6">
    <div
      v-if="loading"
      class="absolute inset-0 z-50 flex select-none flex-col items-center justify-center rounded-xl bg-slate-900/80 backdrop-blur-sm"
    >
      <div class="w-full max-w-md rounded-2xl border border-slate-700 bg-slate-800 p-8 shadow-2xl">
        <div class="mb-2 text-center text-lg font-bold text-emerald-400">{{ loadingText }}</div>
        <div class="mb-3 text-center text-xs text-slate-400">{{ progressStage }}</div>
        <div
          class="mb-4 min-h-10 rounded-lg border border-slate-700/70 bg-slate-900/50 px-3 py-2 font-mono text-xs text-slate-300"
        >
          {{ progressCurrentPath || '正在准备文件列表...' }}
        </div>
        <NProgress
          type="line"
          status="success"
          :percentage="progressPercent"
          :show-indicator="false"
          :height="14"
          rail-color="rgba(51, 65, 85, 0.85)"
          color="#34d399"
          processing
        />
        <div class="mt-3 flex items-center justify-between text-xs text-slate-400">
          <span>大文件处理时长取决于文件体积和压缩级别</span>
          <span class="font-mono text-emerald-300">{{ progressPercent }}%</span>
        </div>
      </div>
    </div>

    <div class="flex items-center justify-between">
      <h2 class="flex items-center gap-2 text-xl font-bold text-slate-100">
        <n-icon class="text-emerald-400"><Package /></n-icon>
        Krate 私有归档
      </h2>
    </div>

    <div class="grid min-h-0 flex-1 grid-cols-2 gap-6">
      <NCard
        title="创建归档"
        class="flex h-full flex-col border-slate-700 bg-slate-800/50"
        content-style="display: flex; flex-direction: column; height: 100%;"
      >
        <div class="mb-4 flex gap-2">
          <NButton dashed class="flex-1" @click="addFiles">
            <template #icon>
              <NIcon><DocumentAdd /></NIcon>
            </template>
            添加文件
          </NButton>
          <NButton dashed class="flex-1" @click="addFolder">
            <template #icon>
              <NIcon><FolderAdd /></NIcon>
            </template>
            添加文件夹
          </NButton>
        </div>

        <div class="mb-3 flex gap-3">
          <div class="flex-1">
            <div class="mb-1 text-xs text-slate-400">压缩等级</div>
            <NSelect v-model:value="compressionLevel" :options="levelOptions" size="small" />
          </div>
          <div class="flex-[2]">
            <div class="mb-1 text-xs text-slate-400">密码（可选）</div>
            <NInput
              v-model:value="packPassword"
              size="small"
              type="password"
              show-password-on="click"
              placeholder="留空则只压缩打包，不启用加密"
            />
          </div>
        </div>

        <div class="mb-3 rounded-lg border border-slate-700/60 bg-slate-900/40 px-3 py-2 text-xs text-slate-400">
          设置密码后，归档会在压缩完成后启用基于密码的分块加密；留空时只做压缩打包，速度更快。
        </div>

        <div
          class="custom-scrollbar flex-1 overflow-y-auto rounded border border-slate-700/50 bg-slate-900/30 p-2"
        >
          <NList v-if="selectedFiles.length > 0">
            <NListItem v-for="(file, index) in selectedFiles" :key="file">
              <div class="group flex items-center justify-between">
                <span class="mr-2 truncate font-mono text-xs text-slate-300">{{ file }}</span>
                <NButton
                  size="tiny"
                  quaternary
                  type="error"
                  class="opacity-0 group-hover:opacity-100"
                  @click="removeFile(index)"
                >
                  <template #icon>
                    <NIcon><Close /></NIcon>
                  </template>
                </NButton>
              </div>
            </NListItem>
          </NList>
          <div v-else class="flex h-full items-center justify-center text-sm text-slate-600">
            请添加文件或文件夹
          </div>
        </div>

        <div class="mt-4">
          <NButton type="primary" block :disabled="!canPack || loading" @click="handlePack">
            生成 .krate 包
          </NButton>
        </div>
      </NCard>

      <NCard
        title="还原归档"
        class="border-slate-700 bg-slate-800/50"
        content-style="display: flex; flex-direction: column; height: 100%;"
      >
        <div class="flex flex-1 flex-col justify-center space-y-6 px-4">
          <div
            class="flex h-32 cursor-pointer flex-col items-center justify-center rounded-xl border-2 border-dashed border-slate-700 transition-colors hover:border-emerald-500/50"
            @click="selectArchive"
          >
            <div v-if="!archivePath" class="text-center text-slate-500">
              <NIcon size="40" class="mb-2"><FolderOpen /></NIcon>
              <div>点击选择 .krate 文件</div>
            </div>
            <div v-else class="px-4 text-center">
              <NIcon size="32" class="mb-1 text-emerald-500"><Package /></NIcon>
              <div class="break-all text-xs text-slate-300">{{ archivePath }}</div>
            </div>
          </div>

          <div class="space-y-2">
            <div class="text-xs text-slate-400">解压位置</div>
            <NInput
              v-model:value="extractDir"
              size="small"
              placeholder="默认解压到同级目录"
              readonly
              @click="selectExtractDir"
            >
              <template #suffix>
                <NIcon><FolderOpen /></NIcon>
              </template>
            </NInput>
          </div>

          <div class="space-y-2">
            <div class="text-xs text-slate-400">解压密码</div>
            <NInput
              v-model:value="unpackPassword"
              size="small"
              type="password"
              show-password-on="click"
              placeholder="如果归档已加密，这里必须填写正确密码"
            />
          </div>

          <div
            v-if="lastExtractedDir"
            class="rounded-lg border border-emerald-500/30 bg-emerald-500/10 px-3 py-3"
          >
            <div class="mb-1 text-xs text-emerald-300">最近一次解压输出目录</div>
            <div class="mb-3 break-all font-mono text-xs text-slate-300">{{ lastExtractedDir }}</div>
            <NButton size="small" secondary type="success" @click="openExtractedDir">
              打开输出目录
            </NButton>
          </div>
        </div>

        <div class="mt-4">
          <NButton
            type="success"
            secondary
            block
            :disabled="!canUnpack || loading"
            @click="handleUnpack"
          >
            <template #icon>
              <NIcon><Deploy /></NIcon>
            </template>
            开始解压
          </NButton>
        </div>
      </NCard>
    </div>
  </div>
</template>
