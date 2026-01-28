<script setup lang="ts">
import { ref, computed } from 'vue'
import imageCompression, { Options } from 'browser-image-compression'
import {
  NButton,
  NSlider,
  NSelect,
  NUpload,
  NUploadDragger,
  NIcon,
  NTag,
  NSpin,
  useMessage,
  type UploadFileInfo,
} from 'naive-ui'
import {
  CloudUpload,
  Download,
  TrashCan,
  Image as ImageIcon,
  CheckmarkFilled,
} from '@vicons/carbon'

const message = useMessage()

// 类型定义
interface ImageItem {
  id: string
  file: File
  originalUrl: string
  compressedUrl?: string
  compressedBlob?: Blob
  status: 'pending' | 'processing' | 'done' | 'error'
  originalSize: number
  compressedSize: number
}

// 状态
const fileList = ref<ImageItem[]>([])
const quality = ref(0.8) // 0-1
// const maxWidth = ref(1920) // 👈 已删除：不再限制尺寸
const outputFormat = ref('original')
const isProcessing = ref(false)

// 格式选项
const formatOptions = [
  { label: '保持原格式', value: 'original' },
  { label: '转为 JPEG', value: 'image/jpeg' },
  { label: '转为 PNG', value: 'image/png' },
  { label: '转为 WebP (推荐)', value: 'image/webp' },
]

// 辅助函数：格式化文件大小
const formatSize = (bytes: number) => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

// 核心：处理上传
const handleFileListChange = (data: { file: UploadFileInfo; fileList: UploadFileInfo[] }) => {
  const nativeFile = data.file.file

  if (!nativeFile) return

  if (!nativeFile.type.startsWith('image/')) {
    message.error('只能上传图片文件')
    data.fileList.pop()
    return
  }

  const item: ImageItem = {
    id: Math.random().toString(36).substring(2),
    file: nativeFile,
    originalUrl: URL.createObjectURL(nativeFile),
    status: 'pending',
    originalSize: nativeFile.size,
    compressedSize: 0,
  }

  fileList.value.push(item)
  compressSingle(item)
}

// 核心：单张压缩逻辑
const compressSingle = async (item: ImageItem) => {
  item.status = 'processing'

  try {
    const options: Options = {
      useWebWorker: true,
      initialQuality: quality.value,
      fileType: outputFormat.value === 'original' ? undefined : outputFormat.value,
    }

    const compressedFile = await imageCompression(item.file, options)

    item.compressedBlob = compressedFile
    item.compressedUrl = URL.createObjectURL(compressedFile)
    item.compressedSize = compressedFile.size
    item.status = 'done'
  } catch (error) {
    console.error(error)
    item.status = 'error'
    message.error(`图片 ${item.file.name} 压缩失败`)
  }
}

// 重新压缩所有
const reCompressAll = () => {
  if (fileList.value.length === 0) return
  isProcessing.value = true
  fileList.value.forEach((item) => {
    compressSingle(item)
  })
  isProcessing.value = false
  message.success('已应用新设置并重新压缩')
}

// 下载单个
const downloadSingle = (item: ImageItem) => {
  if (!item.compressedUrl) return
  const link = document.createElement('a')
  link.href = item.compressedUrl

  let ext = item.file.name.split('.').pop()
  if (outputFormat.value !== 'original') {
    ext = outputFormat.value.split('/')[1]
  }
  const name = item.file.name.split('.').slice(0, -1).join('.')
  link.download = `${name}_min.${ext}`
  link.click()
}

// 清空
const clearAll = () => {
  fileList.value.forEach((item) => {
    URL.revokeObjectURL(item.originalUrl)
    if (item.compressedUrl) URL.revokeObjectURL(item.compressedUrl)
  })
  fileList.value = []
}

// 统计
const totalSavings = computed(() => {
  let original = 0
  let compressed = 0
  fileList.value.forEach((item) => {
    if (item.status === 'done') {
      original += item.originalSize
      compressed += item.compressedSize
    }
  })
  if (original === 0) return 0
  return Math.round(((original - compressed) / original) * 100)
})
</script>

<template>
  <div class="h-full flex flex-col space-y-4">
    <div class="flex items-center justify-between pb-4 border-b border-slate-700/50">
      <div>
        <h2 class="text-2xl font-bold text-slate-100 tracking-tight">图片压缩</h2>
        <p class="text-slate-500 text-sm mt-1">本地无损/有损压缩，支持批量处理</p>
      </div>
      <div class="flex items-center space-x-3">
        <n-tag v-if="totalSavings > 0" type="success" round size="large">
          总体积减少 {{ totalSavings }}%
        </n-tag>
        <div class="h-4 w-[1px] bg-slate-700 mx-2"></div>
        <n-button quaternary size="small" @click="clearAll">
          <template #icon><n-icon :component="TrashCan" /></template>
          清空列表
        </n-button>
      </div>
    </div>

    <div
      class="bg-[#0F172A]/40 p-4 rounded-xl border border-slate-700/50 grid grid-cols-1 md:grid-cols-12 gap-6 items-center"
    >
      <div class="md:col-span-6 space-y-2">
        <div class="flex justify-between text-xs text-slate-400">
          <span>压缩质量</span>
          <span>{{ Math.round(quality * 100) }}%</span>
        </div>
        <n-slider
          v-model:value="quality"
          :step="0.05"
          :min="0.1"
          :max="1.0"
          @update:value="reCompressAll"
        />
      </div>

      <div class="md:col-span-6">
        <div class="text-xs text-slate-400 mb-2">输出格式</div>
        <n-select
          v-model:value="outputFormat"
          :options="formatOptions"
          size="small"
          @update:value="reCompressAll"
        />
      </div>
    </div>

    <div class="flex-1 min-h-0 overflow-y-auto custom-scrollbar pr-2">
      <div
        v-if="fileList.length > 0"
        class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 pb-4"
      >
        <n-upload multiple :show-file-list="false" @change="handleFileListChange" class="h-full">
          <n-upload-dragger
            class="!h-full !bg-slate-800/30 !border-dashed !border-slate-700 hover:!border-cyan-500/50 flex flex-col items-center justify-center p-6 rounded-xl transition-colors cursor-pointer group"
          >
            <div
              class="w-12 h-12 rounded-full bg-slate-800 flex items-center justify-center mb-3 group-hover:bg-cyan-500/20 transition-colors"
            >
              <n-icon
                :size="24"
                :component="CloudUpload"
                class="text-slate-400 group-hover:text-cyan-400"
              />
            </div>
            <div class="text-slate-400 text-sm font-medium">继续添加图片</div>
          </n-upload-dragger>
        </n-upload>

        <div
          v-for="item in fileList"
          :key="item.id"
          class="bg-[#1E293B] rounded-xl border border-slate-700 overflow-hidden flex flex-col shadow-sm hover:border-slate-600 transition-colors"
        >
          <div class="h-32 bg-slate-900/50 relative group">
            <img
              :src="item.compressedUrl || item.originalUrl"
              class="w-full h-full object-contain p-2"
            />

            <div
              class="absolute inset-0 bg-black/50 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center space-x-4"
            >
              <n-button
                circle
                secondary
                type="error"
                @click="fileList = fileList.filter((f) => f.id !== item.id)"
              >
                <template #icon><n-icon :component="TrashCan" /></template>
              </n-button>
            </div>

            <div class="absolute top-2 right-2">
              <n-spin v-if="item.status === 'processing'" size="small" />
              <div
                v-else-if="item.status === 'done'"
                class="bg-green-500 text-white p-1 rounded-full shadow-lg"
              >
                <n-icon :component="CheckmarkFilled" />
              </div>
            </div>
          </div>

          <div class="p-3 flex-1 flex flex-col justify-between">
            <div>
              <div class="text-slate-200 font-medium truncate text-sm mb-1" :title="item.file.name">
                {{ item.file.name }}
              </div>
              <div class="flex items-center space-x-2 text-xs">
                <span class="text-slate-500 line-through">{{ formatSize(item.originalSize) }}</span>
                <span class="text-slate-500">→</span>
                <span class="text-cyan-400 font-bold">{{
                  formatSize(item.compressedSize || 0)
                }}</span>
              </div>
            </div>

            <div class="mt-3 flex items-center justify-between">
              <div class="text-xs font-mono text-green-500 bg-green-500/10 px-2 py-0.5 rounded">
                -{{
                  Math.round(((item.originalSize - item.compressedSize) / item.originalSize) * 100)
                }}%
              </div>
              <n-button
                size="tiny"
                type="primary"
                :disabled="item.status !== 'done'"
                @click="downloadSingle(item)"
              >
                <template #icon><n-icon :component="Download" /></template>
                保存
              </n-button>
            </div>
          </div>
        </div>
      </div>

      <div
        v-else
        class="h-full flex flex-col items-center justify-center p-10 border-2 border-dashed border-slate-700/50 rounded-2xl bg-slate-800/20"
      >
        <n-upload
          multiple
          :show-file-list="false"
          @change="handleFileListChange"
          class="w-full max-w-md"
        >
          <n-upload-dragger class="!bg-transparent !border-0 text-center p-10 cursor-pointer">
            <div
              class="w-20 h-20 rounded-full bg-slate-800 flex items-center justify-center mx-auto mb-6 shadow-xl shadow-cyan-900/10"
            >
              <n-icon :size="40" :component="ImageIcon" class="text-cyan-500" />
            </div>
            <h3 class="text-xl font-bold text-slate-200 mb-2">拖拽图片到这里</h3>
            <p class="text-slate-500">支持 JPG, PNG, WebP · 支持批量处理</p>
          </n-upload-dragger>
        </n-upload>
      </div>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: #334155;
  border-radius: 4px;
}
</style>
