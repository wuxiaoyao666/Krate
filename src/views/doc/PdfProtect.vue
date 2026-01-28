<script setup lang="ts">
import { ref } from 'vue'
import { PDFDocument } from 'pdf-lib'
import {
  NTabs,
  NTabPane,
  NUpload,
  NUploadDragger,
  NInput,
  NButton,
  NIcon,
  useMessage,
  type UploadFileInfo,
} from 'naive-ui'
import { DocumentPdf, Unlocked, TrashCan } from '@vicons/carbon'

const message = useMessage()

// 状态
const activeTab = ref('decrypt') // 默认只展示解密
const currentFile = ref<File | null>(null)
const isProcessing = ref(false)
const decryptPwd = ref('')

// 格式化文件大小
const formatSize = (bytes: number) => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

// ⬇️ 修复：正确的 Upload 回调
const handleUpload = (data: { file: UploadFileInfo; fileList: UploadFileInfo[] }) => {
  const file = data.file.file
  if (!file) return

  // 简单校验
  if (file.type !== 'application/pdf') {
    message.error('请上传 PDF 文件')
    return
  }

  currentFile.value = file
  decryptPwd.value = ''
}

const clearFile = () => {
  currentFile.value = null
  decryptPwd.value = ''
}

// ⬇️ 核心功能：解密 PDF (移除密码)
// 原理：带密码 load，然后无密码 save
const doDecrypt = async () => {
  if (!currentFile.value || !decryptPwd.value) return

  isProcessing.value = true
  try {
    const arrayBuffer = await currentFile.value.arrayBuffer()

    // 1. 尝试带密码加载 PDF
    const pdfDoc = await PDFDocument.load(arrayBuffer, {
      password: decryptPwd.value,
      ignoreEncryption: true,
    })

    // 2. 重新保存（默认是不带密码的）
    const pdfBytes = await pdfDoc.save()

    downloadBlob(pdfBytes, `unlocked_${currentFile.value.name}`)
    message.success('解密成功，已移除密码')
  } catch (err: any) {
    console.error(err)
    if (err.message && err.message.includes('Password')) {
      message.error('密码错误，无法解密')
    } else {
      message.error('解密失败：' + (err.message || '未知错误'))
    }
  } finally {
    isProcessing.value = false
  }
}

// 下载工具函数
const downloadBlob = (data: Uint8Array, filename: string) => {
  const blob = new Blob([data], { type: 'application/pdf' })
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = filename
  link.click()
  URL.revokeObjectURL(url)
}
</script>

<template>
  <div class="h-full flex flex-col space-y-4">
    <div class="flex items-center justify-between pb-4 border-b border-slate-700/50">
      <div>
        <h2 class="text-2xl font-bold text-slate-100 tracking-tight">PDF 工具</h2>
        <p class="text-slate-500 text-sm mt-1">移除 PDF 密码保护</p>
      </div>
    </div>

    <div
      class="flex-1 bg-[#1E293B]/50 rounded-2xl border border-slate-700/50 overflow-hidden flex flex-col"
    >
      <n-tabs
        v-model:value="activeTab"
        type="segment"
        animated
        pane-class="flex-1 h-full p-6 flex flex-col"
        class="h-full"
      >
        <n-tab-pane name="decrypt" tab="解密/移除密码">
          <div class="h-full flex flex-col max-w-2xl mx-auto w-full space-y-8 justify-center">
            <div v-if="!currentFile" class="h-64">
              <n-upload :show-file-list="false" @change="handleUpload" class="h-full">
                <n-upload-dragger
                  class="!h-full !bg-slate-800/50 !border-dashed !border-slate-600 hover:!border-green-500 flex flex-col items-center justify-center rounded-xl transition-all group"
                >
                  <div
                    class="w-16 h-16 rounded-full bg-slate-700 flex items-center justify-center mb-4 group-hover:bg-green-500/20 transition-colors"
                  >
                    <n-icon
                      :size="32"
                      :component="Unlocked"
                      class="text-slate-400 group-hover:text-green-400"
                    />
                  </div>
                  <div class="text-lg font-bold text-slate-200">选择加密的 PDF</div>
                  <div class="text-slate-500 mt-2">永久移除文件密码</div>
                </n-upload-dragger>
              </n-upload>
            </div>

            <div
              v-else
              class="bg-[#0F172A] border border-slate-700 rounded-xl p-6 shadow-xl relative"
            >
              <n-button
                circle
                secondary
                type="error"
                class="absolute top-4 right-4"
                @click="clearFile"
              >
                <template #icon><n-icon :component="TrashCan" /></template>
              </n-button>

              <div class="flex items-center space-x-4 mb-6">
                <div class="w-12 h-12 bg-green-500/20 rounded-lg flex items-center justify-center">
                  <n-icon :size="24" :component="DocumentPdf" class="text-green-400" />
                </div>
                <div>
                  <div class="text-base font-bold text-slate-100">{{ currentFile.name }}</div>
                  <div class="text-sm text-slate-500">{{ formatSize(currentFile.size) }}</div>
                </div>
              </div>

              <div class="space-y-4">
                <div>
                  <label class="text-slate-400 text-sm mb-1 block">输入原密码</label>
                  <n-input
                    v-model:value="decryptPwd"
                    type="password"
                    show-password-on="click"
                    placeholder="请输入该文件的打开密码"
                    size="large"
                  />
                </div>

                <n-button
                  type="primary"
                  color="#22c55e"
                  size="large"
                  block
                  :disabled="!decryptPwd"
                  :loading="isProcessing"
                  @click="doDecrypt"
                >
                  <template #icon><n-icon :component="Unlocked" /></template>
                  {{ isProcessing ? '解密中...' : '解锁并保存副本' }}
                </n-button>
              </div>
            </div>
          </div>
        </n-tab-pane>
      </n-tabs>
    </div>
  </div>
</template>

<style scoped>
:deep(.n-tabs-tab-wrapper) {
  flex: 1;
  display: flex;
  justify-content: center;
}
</style>
