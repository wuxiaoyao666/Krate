<script setup lang="ts">
import { ref } from 'vue'
import {
  NTabs, NTabPane, NInput, NButton, NSpace, NRadioGroup, NRadioButton,
  NUpload, NUploadDragger, NText, NIcon, NImage, useMessage, NCard
} from 'naive-ui'
import type { UploadFileInfo } from 'naive-ui'
import {
  Renew,
  Copy,
  Archive
} from '@vicons/carbon'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'

const message = useMessage()
const activeTab = ref('base64')

// ================= Base64 逻辑 =================
const base64Mode = ref('text') // 'text' | 'image'
const base64Input = ref('')
const base64Output = ref('')

// 处理 Base64 文本编码
const handleBase64Encode = () => {
  try {
    base64Output.value = window.btoa(
      encodeURIComponent(base64Input.value).replace(/%([0-9A-F]{2})/g,
        function toSolidBytes(_, p1) {
          return String.fromCharCode(parseInt(p1, 16))
        })
    )
  } catch (e) {
    message.error('编码失败')
  }
}

// 处理 Base64 文本解码
const handleBase64Decode = () => {
  try {
    const str = window.atob(base64Input.value)
    base64Output.value = decodeURIComponent(
      Array.prototype.map.call(str, function(c) {
        return '%' + ('00' + c.charCodeAt(0).toString(16)).slice(-2)
      }).join('')
    )
  } catch (e) {
    message.error('解码失败：无效的 Base64')
  }
}

// Base64 图片处理
const imageBase64Preview = ref('')

// 2. 修复类型报错：使用正确的参数类型
const handleImageUpload = (data: { file: UploadFileInfo }) => {
  const fileObj = data.file.file
  if (!fileObj) return false

  const reader = new FileReader()
  reader.onload = (e) => {
    const result = e.target?.result as string
    base64Output.value = result
    imageBase64Preview.value = result
  }
  reader.readAsDataURL(fileObj)
  return false
}

const handleBase64InputChange = (val: string) => {
  base64Input.value = val
  if (base64Mode.value === 'image' && val.startsWith('data:image')) {
    imageBase64Preview.value = val
  }
}

// ================= URL 编码逻辑 =================
const urlInput = ref('')
const urlOutput = ref('')

const handleUrlEncode = () => {
  urlOutput.value = encodeURIComponent(urlInput.value)
}

const handleUrlDecode = () => {
  try {
    urlOutput.value = decodeURIComponent(urlInput.value)
  } catch (e) {
    message.error('解码失败')
  }
}

// ================= Unicode 逻辑 =================
const unicodeInput = ref('')
const unicodeOutput = ref('')

const handleUnicodeEscape = () => {
  unicodeOutput.value = unicodeInput.value.split('').map(char => {
    const code = char.charCodeAt(0)
    return code > 127 ? '\\u' + code.toString(16).padStart(4, '0') : char
  }).join('')
}

const handleUnicodeUnescape = () => {
  try {
    unicodeOutput.value = JSON.parse(`"${unicodeInput.value}"`)
  } catch (e) {
    unicodeOutput.value = unicodeInput.value.replace(/\\u[\dA-F]{4}/gi, (match) => {
      return String.fromCharCode(parseInt(match.replace(/\\u/g, ''), 16))
    })
  }
}

// ================= 通用工具 =================
const copyToClipboard = async (text: string) => {
  if (!text) {
    message.warning('没有内容可复制')
    return
  }
  try {
    await writeText(text)
    message.success('已复制')
  } catch (e) {
    message.error(`复制失败 ${e}`)
  }
}

const clearAll = (type: string) => {
  if (type === 'base64') {
    base64Input.value = ''
    base64Output.value = ''
    imageBase64Preview.value = ''
  } else if (type === 'url') {
    urlInput.value = ''
    urlOutput.value = ''
  } else if (type === 'unicode') {
    unicodeInput.value = ''
    unicodeOutput.value = ''
  }
}
</script>

<template>
  <div class="h-full flex flex-col p-6 space-y-4">
    <div class="flex justify-between items-center">
      <h2 class="text-xl font-bold text-slate-100">编码解码</h2>
    </div>

    <NCard class="flex-1 bg-slate-800/50 border-slate-700"
           content-style="height: 100%; display: flex; flex-direction: column;">
      <NTabs type="line" animated v-model:value="activeTab" class="flex-1 h-full">

        <NTabPane name="base64" tab="Base64">
          <div class="flex flex-col h-full space-y-4">
            <div class="flex justify-center mb-2">
              <NRadioGroup v-model:value="base64Mode">
                <NRadioButton value="text">文本模式</NRadioButton>
                <NRadioButton value="image">图片 Base64</NRadioButton>
              </NRadioGroup>
            </div>

            <template v-if="base64Mode === 'text'">
              <div class="flex-1 flex flex-col">
                <span class="text-xs text-slate-500 mb-1">输入内容</span>
                <NInput
                  v-model:value="base64Input"
                  type="textarea"
                  placeholder="请输入要处理的内容..."
                  class="flex-1 bg-slate-900/50"
                  :resizable="false"
                />
              </div>

              <div class="py-2 flex justify-center">
                <NSpace>
                  <NButton type="primary" @click="handleBase64Encode">编码 ↓</NButton>
                  <NButton type="info" @click="handleBase64Decode">解码 ↑</NButton>
                  <NButton secondary type="warning" @click="clearAll('base64')">
                    <template #icon>
                      <NIcon>
                        <Renew />
                      </NIcon>
                    </template>
                    清空
                  </NButton>
                </NSpace>
              </div>

              <div class="flex-1 flex flex-col">
                <div class="flex justify-between items-center mb-1">
                  <span class="text-xs text-slate-500">结果输出</span>
                  <NButton size="tiny" secondary type="success" @click="copyToClipboard(base64Output)">
                    <template #icon>
                      <NIcon>
                        <Copy />
                      </NIcon>
                    </template>
                    复制结果
                  </NButton>
                </div>
                <NInput
                  v-model:value="base64Output"
                  type="textarea"
                  placeholder="结果输出..."
                  readonly
                  class="flex-1 bg-slate-900/50 font-mono"
                  :resizable="false"
                />
              </div>
            </template>

            <template v-else>
              <div class="grid grid-cols-2 gap-4 h-full">
                <div class="flex flex-col space-y-4">
                  <NUpload
                    multiple
                    directory-dnd
                    :show-file-list="false"
                    @change="handleImageUpload"
                    class="h-48"
                  >
                    <NUploadDragger
                      class="h-full flex flex-col items-center justify-center bg-slate-900/30 border-slate-700">
                      <div style="margin-bottom: 12px">
                        <NIcon size="48" :depth="3">
                          <Archive />
                        </NIcon>
                      </div>
                      <NText style="font-size: 16px">点击或拖拽图片</NText>
                    </NUploadDragger>
                  </NUpload>

                  <div class="flex-1 flex flex-col">
                    <span class="text-xs text-slate-500 mb-1">Base64 字符串</span>
                    <NInput
                      :value="base64Input"
                      @update:value="handleBase64InputChange"
                      type="textarea"
                      placeholder="或在此粘贴 Base64..."
                      class="flex-1 bg-slate-900/50 font-mono text-xs"
                      :resizable="false"
                    />
                  </div>
                </div>

                <div class="flex flex-col space-y-4 h-full">
                  <div
                    class="h-48 flex items-center justify-center bg-slate-900/50 rounded border border-slate-700 overflow-hidden">
                    <NImage
                      v-if="imageBase64Preview"
                      :src="imageBase64Preview"
                      object-fit="contain"
                      class="max-h-full max-w-full"
                    />
                    <span v-else class="text-slate-500 text-sm">图片预览区域</span>
                  </div>

                  <div class="flex-1 flex flex-col">
                    <div class="flex justify-between items-center mb-1">
                      <span class="text-xs text-slate-500">转换结果</span>
                      <NButton size="tiny" secondary type="success" @click="copyToClipboard(base64Output)">
                        <template #icon>
                          <NIcon>
                            <Copy />
                          </NIcon>
                        </template>
                        复制结果
                      </NButton>
                    </div>
                    <NInput
                      v-model:value="base64Output"
                      type="textarea"
                      placeholder="Base64 结果..."
                      readonly
                      class="flex-1 bg-slate-900/50 font-mono text-xs"
                      :resizable="false"
                    />
                  </div>
                </div>
              </div>
            </template>
          </div>
        </NTabPane>

        <NTabPane name="url" tab="URL Encode">
          <div class="flex flex-col h-full space-y-4">
            <div class="flex-1 flex flex-col">
              <span class="text-xs text-slate-500 mb-1">输入 URL</span>
              <NInput
                v-model:value="urlInput"
                type="textarea"
                placeholder="请输入 URL..."
                class="flex-1 bg-slate-900/50"
                :resizable="false"
              />
            </div>

            <div class="py-2 flex justify-center">
              <NSpace>
                <NButton type="primary" @click="handleUrlEncode">编码 ↓</NButton>
                <NButton type="info" @click="handleUrlDecode">解码 ↑</NButton>
                <NButton secondary type="warning" @click="clearAll('url')">
                  <template #icon>
                    <NIcon>
                      <Renew />
                    </NIcon>
                  </template>
                  清空
                </NButton>
              </NSpace>
            </div>

            <div class="flex-1 flex flex-col">
              <div class="flex justify-between items-center mb-1">
                <span class="text-xs text-slate-500">结果输出</span>
                <NButton size="tiny" secondary type="success" @click="copyToClipboard(urlOutput)">
                  <template #icon>
                    <NIcon>
                      <Copy />
                    </NIcon>
                  </template>
                  复制结果
                </NButton>
              </div>
              <NInput
                v-model:value="urlOutput"
                type="textarea"
                placeholder="结果..."
                readonly
                class="flex-1 bg-slate-900/50"
                :resizable="false"
              />
            </div>
          </div>
        </NTabPane>

        <NTabPane name="unicode" tab="Unicode">
          <div class="flex flex-col h-full space-y-4">
            <div class="flex-1 flex flex-col">
              <span class="text-xs text-slate-500 mb-1">输入文本</span>
              <NInput
                v-model:value="unicodeInput"
                type="textarea"
                placeholder="请输入文本 (例如: 你好)..."
                class="flex-1 bg-slate-900/50"
                :resizable="false"
              />
            </div>

            <div class="py-2 flex justify-center">
              <NSpace>
                <NButton type="primary" @click="handleUnicodeEscape">转义 ↓</NButton>
                <NButton type="info" @click="handleUnicodeUnescape">反转义 ↑</NButton>
                <NButton secondary type="warning" @click="clearAll('unicode')">
                  <template #icon>
                    <NIcon>
                      <Renew />
                    </NIcon>
                  </template>
                  清空
                </NButton>
              </NSpace>
            </div>

            <div class="flex-1 flex flex-col">
              <div class="flex justify-between items-center mb-1">
                <span class="text-xs text-slate-500">结果输出</span>
                <NButton size="tiny" secondary type="success" @click="copyToClipboard(unicodeOutput)">
                  <template #icon>
                    <NIcon>
                      <Copy />
                    </NIcon>
                  </template>
                  复制结果
                </NButton>
              </div>
              <NInput
                v-model:value="unicodeOutput"
                type="textarea"
                placeholder="结果 (例如: \u4f60\u597d)..."
                readonly
                class="flex-1 bg-slate-900/50 font-mono"
                :resizable="false"
              />
            </div>
          </div>
        </NTabPane>

      </NTabs>
    </NCard>
  </div>
</template>