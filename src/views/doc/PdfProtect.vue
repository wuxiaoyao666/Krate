<script setup lang="ts">
import { ref } from 'vue'
import { Command } from '@tauri-apps/plugin-shell'
import { open, save } from '@tauri-apps/plugin-dialog'
import { NTabs, NTabPane, NInput, NButton, NIcon, useMessage } from 'naive-ui'
import { Locked, Unlocked, TrashCan, DocumentPdf, ArrowsVertical } from '@vicons/carbon'

const message = useMessage()
const isProcessing = ref(false)
const activeTab = ref('encrypt')
const currentFile = ref<{ name: string; path: string } | null>(null)
const password = ref('')

const executeExtension = async (module: string, func: string, payload: object) => {
  try {
    const cmd = Command.sidecar('bin/krate_extension', [module, func, JSON.stringify(payload)])
    const output = await cmd.execute()

    if (!output.stdout) {
      throw new Error(output.stderr || 'Python 进程未返回数据')
    }

    const res = JSON.parse(output.stdout)
    if (res.status === 'success') return res

    throw new Error(res.msg || '未知错误')
  } catch (e: any) {
    console.error('Python Error:', e)
    throw new Error(e.message || '调用核心组件失败')
  }
}

const selectPdfFile = async () => {
  const selected = await open({
    multiple: false,
    title: '选择 PDF 文件',
    filters: [{ name: 'PDF', extensions: ['pdf'] }]
  })

  if (selected) {
    const name = selected.split(/[\\/]/).pop() || 'Unknown.pdf'
    currentFile.value = { name, path: selected }
    password.value = ''
  }
}

const selectWordFile = async () => {
  const selected = await open({
    multiple: false,
    title: '选择 Word 文件',
    filters: [{ name: 'Word', extensions: ['doc', 'docx'] }]
  })

  if (selected) {
    const name = selected.split(/[\\/]/).pop() || 'Unknown.docx'
    currentFile.value = { name, path: selected }
    password.value = ''
  }
}

const clearFile = () => {
  currentFile.value = null
  password.value = ''
}

const doEncrypt = async () => {
  if (!currentFile.value || !password.value) return
  isProcessing.value = true

  try {
    const savePath = await save({
      defaultPath: currentFile.value.path.replace('.pdf', '_encrypted.pdf'),
      filters: [{ name: 'PDF', extensions: ['pdf'] }]
    })
    if (!savePath) return

    await executeExtension('pdf_ops', 'encrypt_pdf', {
      input: currentFile.value.path,
      output: savePath,
      password: password.value
    })

    message.success('加密成功！')
    clearFile()
  } catch (err: any) {
    message.error('失败: ' + err.message)
  } finally {
    isProcessing.value = false
  }
}

const doDecrypt = async () => {
  if (!currentFile.value || !password.value) return
  isProcessing.value = true

  try {
    const savePath = await save({
      defaultPath: currentFile.value.path.replace('.pdf', '_unlocked.pdf'),
      filters: [{ name: 'PDF', extensions: ['pdf'] }]
    })
    if (!savePath) return

    await executeExtension('pdf_ops', 'decrypt_pdf', {
      input: currentFile.value.path,
      output: savePath,
      password: password.value
    })

    message.success('解密成功！')
    clearFile()
  } catch (err: any) {
    message.error('失败: ' + err.message)
  } finally {
    isProcessing.value = false
  }
}

const doWordToPdf = async () => {
  if (!currentFile.value) return
  isProcessing.value = true

  try {
    const defaultPath = currentFile.value.path.replace(/\.(doc|docx)$/i, '.pdf')
    const savePath = await save({
      defaultPath,
      filters: [{ name: 'PDF', extensions: ['pdf'] }]
    })
    if (!savePath) return

    await executeExtension('word_ops', 'convert_word_to_pdf', {
      input: currentFile.value.path,
      output: savePath
    })

    message.success('Word 转换完成！')
    clearFile()
  } catch (err: any) {
    message.error('失败: ' + err.message)
  } finally {
    isProcessing.value = false
  }
}
</script>

<template>
  <div class="h-full flex flex-col space-y-4">
    <div class="flex items-center justify-between pb-4 border-b border-slate-700/50">
      <div>
        <h2 class="text-2xl font-bold text-slate-100 tracking-tight">PDF 工具箱</h2>
        <p class="text-slate-500 text-sm mt-1">PDF 加解密与 Word 转 PDF</p>
      </div>
    </div>

    <div class="flex-1 bg-[#1E293B]/50 rounded-2xl border border-slate-700/50 overflow-hidden flex flex-col">
      <n-tabs v-model:value="activeTab" type="segment" animated pane-class="flex-1 h-full p-6 flex flex-col" class="h-full">

        <n-tab-pane name="encrypt" tab="加密 PDF">
          <div class="h-full flex flex-col max-w-2xl mx-auto w-full space-y-8 justify-center">
            <div v-if="!currentFile" class="h-64 border-2 border-dashed border-slate-700 rounded-xl hover:border-cyan-500 transition-all cursor-pointer flex flex-col items-center justify-center bg-slate-800/30 group" @click="selectPdfFile">
              <div class="w-16 h-16 rounded-full bg-slate-700 flex items-center justify-center mb-4 group-hover:bg-cyan-500/20 transition-colors">
                <n-icon :size="32" :component="Locked" class="text-slate-400 group-hover:text-cyan-400" />
              </div>
              <div class="text-lg font-bold text-slate-200">点击选择 PDF</div>
            </div>

            <div v-else class="bg-[#0F172A] border border-slate-700 rounded-xl p-8 shadow-xl relative">
              <n-button circle secondary type="error" class="absolute top-4 right-4" @click="clearFile">
                <template #icon><n-icon :component="TrashCan" /></template>
              </n-button>

              <div class="flex items-center space-x-5 mb-8">
                <div class="w-14 h-14 bg-red-500/20 rounded-xl flex items-center justify-center shrink-0">
                  <n-icon :size="28" :component="DocumentPdf" class="text-red-400" />
                </div>
                <div class="flex-1 min-w-0">
                  <div class="text-lg font-bold text-slate-100 truncate">{{ currentFile.name }}</div>
                  <div class="text-xs text-slate-500 font-mono mt-1 opacity-60">{{ currentFile.path }}</div>
                </div>
              </div>

              <div class="space-y-5">
                <n-input v-model:value="password" type="password" show-password-on="click" placeholder="设置打开密码" size="large" />
                <n-button type="primary" size="large" block :disabled="!password" :loading="isProcessing" @click="doEncrypt">
                  <template #icon><n-icon :component="Locked" /></template>
                  加密并导出
                </n-button>
              </div>
            </div>
          </div>
        </n-tab-pane>

        <n-tab-pane name="decrypt" tab="移除密码">
          <div class="h-full flex flex-col max-w-2xl mx-auto w-full space-y-8 justify-center">
            <div v-if="!currentFile" class="h-64 border-2 border-dashed border-slate-700 rounded-xl hover:border-green-500 transition-all cursor-pointer flex flex-col items-center justify-center bg-slate-800/30 group" @click="selectPdfFile">
              <div class="w-16 h-16 rounded-full bg-slate-700 flex items-center justify-center mb-4 group-hover:bg-green-500/20 transition-colors">
                <n-icon :size="32" :component="Unlocked" class="text-slate-400 group-hover:text-green-400" />
              </div>
              <div class="text-lg font-bold text-slate-200">选择加密 PDF</div>
            </div>

            <div v-else class="bg-[#0F172A] border border-slate-700 rounded-xl p-8 shadow-xl relative">
              <n-button circle secondary type="error" class="absolute top-4 right-4" @click="clearFile">
                <template #icon><n-icon :component="TrashCan" /></template>
              </n-button>

              <div class="flex items-center space-x-5 mb-8">
                <div class="w-14 h-14 bg-green-500/20 rounded-xl flex items-center justify-center shrink-0">
                  <n-icon :size="28" :component="DocumentPdf" class="text-green-400" />
                </div>
                <div class="flex-1 min-w-0">
                  <div class="text-lg font-bold text-slate-100 truncate">{{ currentFile.name }}</div>
                </div>
              </div>

              <div class="space-y-5">
                <n-input v-model:value="password" type="password" show-password-on="click" placeholder="输入原密码" size="large" />
                <n-button type="primary" color="#22c55e" size="large" block :disabled="!password" :loading="isProcessing" @click="doDecrypt">
                  <template #icon><n-icon :component="Unlocked" /></template>
                  解锁并导出
                </n-button>
              </div>
            </div>
          </div>
        </n-tab-pane>

        <n-tab-pane name="word" tab="Word 转 PDF">
          <div class="h-full flex flex-col max-w-2xl mx-auto w-full space-y-8 justify-center">
            <div v-if="!currentFile" class="h-64 border-2 border-dashed border-slate-700 rounded-xl hover:border-violet-500 transition-all cursor-pointer flex flex-col items-center justify-center bg-slate-800/30 group" @click="selectWordFile">
              <div class="w-16 h-16 rounded-full bg-slate-700 flex items-center justify-center mb-4 group-hover:bg-violet-500/20 transition-colors">
                <n-icon :size="32" :component="ArrowsVertical" class="text-slate-400 group-hover:text-violet-400" />
              </div>
              <div class="text-lg font-bold text-slate-200">选择 Word 文件（.doc / .docx）</div>
            </div>

            <div v-else class="bg-[#0F172A] border border-slate-700 rounded-xl p-8 shadow-xl relative">
              <n-button circle secondary type="error" class="absolute top-4 right-4" @click="clearFile">
                <template #icon><n-icon :component="TrashCan" /></template>
              </n-button>

              <div class="flex items-center space-x-5 mb-8">
                <div class="w-14 h-14 bg-violet-500/20 rounded-xl flex items-center justify-center shrink-0">
                  <n-icon :size="28" :component="ArrowsVertical" class="text-violet-400" />
                </div>
                <div class="flex-1 min-w-0">
                  <div class="text-lg font-bold text-slate-100 truncate">{{ currentFile.name }}</div>
                  <div class="text-xs text-slate-500 font-mono mt-1 opacity-60">{{ currentFile.path }}</div>
                </div>
              </div>

              <n-button type="primary" color="#8b5cf6" size="large" block :loading="isProcessing" @click="doWordToPdf">
                <template #icon><n-icon :component="ArrowsVertical" /></template>
                转换并导出 PDF
              </n-button>
            </div>
          </div>
        </n-tab-pane>

      </n-tabs>
    </div>
  </div>
</template>
