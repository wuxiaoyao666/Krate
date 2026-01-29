<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import QRCode from 'qrcode'
import { NInput, NInputNumber, NColorPicker, NSelect, NButton, NIcon, NSlider, useMessage } from 'naive-ui'
import { QrCode, Download, Reset, ImageCopy } from '@vicons/carbon'
import { save } from '@tauri-apps/plugin-dialog'
import { writeFile } from '@tauri-apps/plugin-fs'
import { Image as TauriImage } from '@tauri-apps/api/image'
import { writeImage } from '@tauri-apps/plugin-clipboard-manager'

const message = useMessage()

// 配置项
const content = ref('https://wuxiaoyao.online')
const size = ref(300)
const margin = ref(2)
const darkColor = ref('#000000')
const lightColor = ref('#ffffff')
const errorLevel = ref<'L' | 'M' | 'Q' | 'H'>('M')

// 生成结果
const qrDataUrl = ref('')

// 容错率选项
const errorOptions = [
  { label: '低 (7%) - 适合无遮挡', value: 'L' },
  { label: '中 (15%) - 标准', value: 'M' },
  { label: '高 (25%)', value: 'Q' },
  { label: '极高 (30%) - 适合中间加Logo', value: 'H' }
]

// 生成二维码核心逻辑
const generateQR = async () => {
  if (!content.value) return
  try {
    qrDataUrl.value = await QRCode.toDataURL(content.value, {
      width: size.value,
      margin: margin.value,
      color: {
        dark: darkColor.value,
        light: lightColor.value
      },
      errorCorrectionLevel: errorLevel.value
    })
  } catch (err) {
    console.error(err)
  }
}

// 监听所有配置变化，自动重绘
watch([content, size, margin, darkColor, lightColor, errorLevel], () => {
  generateQR()
})

onMounted(() => {
  generateQR()
})

// 重置配置
const resetConfig = () => {
  content.value = 'https://wuxiaoyao.online'
  size.value = 300
  margin.value = 2
  darkColor.value = '#000000'
  lightColor.value = '#ffffff'
  errorLevel.value = 'M'
}

// 保存文件逻辑 (Tauri Native)
const saveImage = async () => {
  if (!qrDataUrl.value) return

  try {
    const filePath = await save({
      filters: [{ name: 'PNG Image', extensions: ['png'] }],
      defaultPath: 'qrcode.png'
    })

    if (!filePath) return

    // Base64 转 Uint8Array
    const base64Data = qrDataUrl.value.replace(/^data:image\/\w+;base64,/, '')
    const buffer = Uint8Array.from(atob(base64Data), c => c.charCodeAt(0))

    // 写入文件
    await writeFile(filePath, buffer)
    message.success('保存成功！')
  } catch (e: any) {
    message.error('保存失败: ' + e)
  }
}

// 复制到剪贴板
const copyImage = async () => {
  if (!qrDataUrl.value) return

  try {
    // 1. 处理 Base64：去掉头部的 "data:image/png;base64,"
    const base64Str = qrDataUrl.value.split(',')[1]

    // 2. 转换成二进制数组
    const binaryString = atob(base64Str)
    const len = binaryString.length
    const bytes = new Uint8Array(len)
    for (let i = 0; i < len; i++) {
      bytes[i] = binaryString.charCodeAt(i)
    }

    // 3. 构建 Tauri 的 Image 对象
    const image = await TauriImage.fromBytes(bytes)

    // 4. 调用原生接口写入剪贴板
    await writeImage(image)

    message.success('已复制图片到剪贴板！')
  } catch (e: any) {
    const errorMsg = typeof e === 'string' ? e : (e.message || JSON.stringify(e))
    message.error('复制失败: ' + errorMsg)
    console.error(e)
  }
}
</script>

<template>
  <div class="h-full flex flex-col max-w-5xl mx-auto p-6 space-y-6">
    <div class="flex items-center justify-between">
      <div>
        <h2 class="text-2xl font-bold text-slate-100 flex items-center gap-2">
          <n-icon :component="QrCode" class="text-green-400" />
          二维码生成器
        </h2>
        <p class="text-slate-500 text-sm mt-1">实时预览 · 矢量级清晰度 · 自定义配色</p>
      </div>
      <n-button secondary size="small" @click="resetConfig">
        <template #icon>
          <n-icon :component="Reset" />
        </template>
        重置默认
      </n-button>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-3 gap-8 flex-1">

      <div class="lg:col-span-2 space-y-6">
        <div class="bg-[#1E293B] p-5 rounded-xl border border-slate-700 shadow-sm">
          <label class="text-slate-400 text-sm mb-2 block">内容文本 / URL</label>
          <n-input
            v-model:value="content"
            type="textarea"
            placeholder="请输入想要转换的文字或链接..."
            :autosize="{ minRows: 3, maxRows: 5 }"
            class="text-base"
          />
        </div>

        <div
          class="bg-[#1E293B] p-5 rounded-xl border border-slate-700 shadow-sm grid grid-cols-1 md:grid-cols-2 gap-6">

          <div class="space-y-4">
            <h3 class="text-slate-200 font-medium border-b border-slate-700 pb-2">配色方案</h3>
            <div class="flex items-center justify-between">
              <span class="text-slate-400 text-sm">前景色 (码)</span>
              <n-color-picker v-model:value="darkColor" :show-alpha="false" size="small" class="w-32" />
            </div>
            <div class="flex items-center justify-between">
              <span class="text-slate-400 text-sm">背景色</span>
              <n-color-picker v-model:value="lightColor" :show-alpha="false" size="small" class="w-32" />
            </div>
          </div>

          <div class="space-y-4">
            <h3 class="text-slate-200 font-medium border-b border-slate-700 pb-2">规格设置</h3>
            <div class="space-y-1">
              <div class="flex justify-between text-xs text-slate-400">
                <span>尺寸 ({{ size }}px)</span>
              </div>
              <n-slider v-model:value="size" :min="100" :max="1000" :step="10" />
            </div>

            <div class="flex items-center gap-4">
              <div class="flex-1">
                <span class="text-slate-400 text-xs block mb-1">边距</span>
                <n-input-number v-model:value="margin" size="small" :min="0" :max="10" />
              </div>
              <div class="flex-1">
                <span class="text-slate-400 text-xs block mb-1">容错率</span>
                <n-select v-model:value="errorLevel" :options="errorOptions" size="small" />
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="flex flex-col gap-4">
        <div
          class="bg-white/5 rounded-2xl border border-slate-700 flex items-center justify-center p-8 min-h-[300px] relative overflow-hidden group">
          <div class="absolute inset-0 opacity-30"
               style="background-image: radial-gradient(#475569 1px, transparent 1px); background-size: 10px 10px;"></div>

          <img v-if="qrDataUrl" :src="qrDataUrl"
               class="relative z-10 shadow-2xl rounded-sm max-w-full max-h-[300px] border-4 border-white"
               alt="QR Code Preview" />
          <div v-else class="text-slate-500 z-10">等待输入...</div>
        </div>

        <div class="grid grid-cols-2 gap-3">
          <n-button block secondary type="primary" @click="copyImage">
            <template #icon>
              <n-icon :component="ImageCopy" />
            </template>
            复制图片
          </n-button>
          <n-button block type="primary" @click="saveImage">
            <template #icon>
              <n-icon :component="Download" />
            </template>
            保存文件
          </n-button>
        </div>

        <div class="text-xs text-slate-500 text-center px-4">
          <p>容错率说明：如果要在二维码中间贴 Logo，请至少选择 "高" 或 "极高"。</p>
        </div>
      </div>

    </div>
  </div>
</template>