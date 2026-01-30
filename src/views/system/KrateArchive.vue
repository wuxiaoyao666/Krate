<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, UnlistenFn } from '@tauri-apps/api/event'
import { open, save } from '@tauri-apps/plugin-dialog'
import {
  NButton,
  NIcon,
  NCard,
  NList,
  NListItem,
  useMessage,
  NTag,
  NInput,
  NSelect,
  NProgress,
  NModal,
} from 'naive-ui'
import { Package, Deploy, DocumentAdd, FolderAdd, Close, FolderOpen } from '@vicons/carbon'

const message = useMessage()

// 后端 emit 的 payload：KrateProgress
type Phase = 'scan' | 'pack' | 'unpack'
interface KrateProgress {
  phase: Phase
  current: number
  total: number
  message: string
}

// ========== Pack ==========
const selectedFiles = ref<string[]>([])
const packing = ref(false)
const packPassword = ref('')
const compressionLevel = ref<number>(9)

const levelOptions = [
  { label: '最快 (1)', value: 1 },
  { label: '平衡 (6)', value: 6 },
  { label: '最高压缩 (9)', value: 9 },
]

// ========== Unpack ==========
const archivePath = ref('')
const extractDir = ref('')
const unpacking = ref(false)
const unpackPassword = ref('')

// ========== Progress ==========
const showProgress = ref(false)
const progressPercent = ref(0)
const progressText = ref('')
const progressDetail = ref('')
const progressPhase = ref<Phase>('pack')

let fakeTimer: number | null = null
const startFakeProgress = () => {
  stopFakeProgress()
  fakeTimer = window.setInterval(() => {
    progressPercent.value = Math.min(95, progressPercent.value + 1)
  }, 300)
}
const stopFakeProgress = () => {
  if (fakeTimer) {
    clearInterval(fakeTimer)
    fakeTimer = null
  }
}

const resetProgress = (phase: Phase) => {
  showProgress.value = true
  progressPhase.value = phase
  progressPercent.value = 0
  progressDetail.value = ''
  progressText.value =
    phase === 'scan' ? '扫描文件...' : phase === 'pack' ? '正在打包...' : '正在解压...'
  startFakeProgress()
}

const closeProgress = () => {
  stopFakeProgress()
  showProgress.value = false
}

// ========== helpers ==========
const dirname = (p: string) => {
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

const canPack = computed(() => selectedFiles.value.length > 0 && !packing.value)
const canUnpack = computed(() => !!archivePath.value && !!extractDir.value && !unpacking.value)

// ========== Pack ==========
const handlePack = async () => {
  if (!selectedFiles.value.length) return

  try {
    const savePath = await save({
      defaultPath: 'data.krate',
      filters: [{ name: 'Krate Package', extensions: ['krate'] }],
    })
    if (!savePath) return

    packing.value = true
    resetProgress('scan') // 后端会先发 scan 再 pack

    // ✅ 重点：Tauri 前端传参使用 camelCase
    await invoke('create_archive', {
      inputs: selectedFiles.value,
      outputPath: savePath, // ✅ 必须 outputPath
      password: packPassword.value.trim() || null,
      gzipLevel: compressionLevel.value, // ✅ 对应 Rust gzip_level
    })

    message.success('打包成功！已生成 .krate 文件')
    selectedFiles.value = []
    packPassword.value = ''
    closeProgress()
  } catch (e: any) {
    message.error('打包失败: ' + (e?.message ?? e))
    closeProgress()
  } finally {
    packing.value = false
  }
}

// ========== Unpack ==========
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
    unpacking.value = true
    resetProgress('unpack')

    // ✅ 同样 camelCase
    await invoke('extract_archive', {
      archivePath: archivePath.value, // ✅ archivePath
      outputDir: extractDir.value, // ✅ outputDir
      password: unpackPassword.value.trim() || null,
    })

    message.success('解压成功！')
    closeProgress()
  } catch (e: any) {
    message.error('解压失败: ' + (e?.message ?? e))
    closeProgress()
  } finally {
    unpacking.value = false
  }
}

// ========== listen progress ==========
let unlisten: UnlistenFn | null = null

onMounted(async () => {
  unlisten = await listen<KrateProgress>('krate://progress', (ev) => {
    const p = ev.payload

    if (!showProgress.value) resetProgress(p.phase)

    progressPhase.value = p.phase
    progressDetail.value = p.message || ''

    if (p.phase === 'scan') progressText.value = '扫描文件...'
    if (p.phase === 'pack') progressText.value = '正在打包...'
    if (p.phase === 'unpack') progressText.value = '正在解压...'

    if (p.total && p.total > 0) {
      stopFakeProgress()
      const percent = Math.round((p.current / p.total) * 100)
      progressPercent.value = Math.max(0, Math.min(100, percent))
    } else {
      if (!fakeTimer) startFakeProgress()
    }

    // 完成：current>=total
    if (p.total > 0 && p.current >= p.total) {
      stopFakeProgress()
      progressPercent.value = 100
      progressText.value = '完成'
    }
  })
})

onBeforeUnmount(() => {
  unlisten?.()
  stopFakeProgress()
})
</script>

<template>
  <div class="h-full flex flex-col p-6 space-y-4">
    <div class="flex justify-between items-center">
      <h2 class="text-xl font-bold text-slate-100 flex items-center gap-2">
        <n-icon class="text-emerald-400"><Package /></n-icon>
        Krate 私有归档
      </h2>
      <NTag type="warning" size="small" :bordered="false">仅限 Krate 内部使用的 .krate 格式</NTag>
    </div>

    <div class="flex-1 grid grid-cols-2 gap-6 min-h-0">
      <!-- Pack -->
      <NCard
        title="创建归档 (Pack)"
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
          <div class="flex-[2]">
            <div class="text-xs text-slate-400 mb-1">密码（可选，加密后只有本软件可解）</div>
            <NInput
              v-model:value="packPassword"
              size="small"
              type="password"
              show-password-on="click"
              placeholder="留空 = 不加密"
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
            拖拽或点击上方按钮添加内容
          </div>
        </div>

        <div class="mt-4">
          <NButton type="primary" block @click="handlePack" :loading="packing" :disabled="!canPack">
            生成 .krate 包
          </NButton>
        </div>
      </NCard>

      <!-- Unpack -->
      <NCard
        title="还原归档 (Unpack)"
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
              placeholder="选择解压目录..."
              v-model:value="extractDir"
              readonly
              @click="selectExtractDir"
            >
              <template #suffix>
                <NIcon class="cursor-pointer"><FolderOpen /></NIcon>
              </template>
            </NInput>
          </div>

          <div class="space-y-2">
            <div class="text-xs text-slate-400">密码（如果包加密）</div>
            <NInput
              v-model:value="unpackPassword"
              size="small"
              type="password"
              show-password-on="click"
              placeholder="加密包必填"
            />
          </div>
        </div>

        <div class="mt-4">
          <NButton
            type="success"
            secondary
            block
            @click="handleUnpack"
            :loading="unpacking"
            :disabled="!canUnpack"
          >
            <template #icon
              ><NIcon><Deploy /></NIcon
            ></template>
            开始解压
          </NButton>
        </div>
      </NCard>
    </div>

    <!-- Progress Modal -->
    <NModal v-model:show="showProgress" :mask-closable="false" preset="card" style="width: 520px">
      <div class="text-slate-100 font-semibold mb-2">{{ progressText }}</div>
      <NProgress type="line" :percentage="progressPercent" indicator-placement="inside" />
      <div class="text-xs text-slate-400 mt-2 break-all">{{ progressDetail }}</div>
      <div class="text-xs text-slate-500 mt-1">提示：大文件压缩/解压需要时间，进度会持续更新</div>
    </NModal>
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
