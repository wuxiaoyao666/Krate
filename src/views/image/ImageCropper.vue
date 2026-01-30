<template>
  <div class="h-full flex flex-col p-4 gap-3">
    <!-- 顶部工具条 -->
    <div class="flex flex-wrap items-center gap-2">
      <n-button type="primary" @click="pickImage">选择图片</n-button>
      <n-button :disabled="!hasImage" @click="resetCrop">重置</n-button>

      <div class="h-5 w-px bg-gray-200 mx-1" />

      <n-button :disabled="!hasImage" @click="zoom(0.15)">放大</n-button>
      <n-button :disabled="!hasImage" @click="zoom(-0.15)">缩小</n-button>
      <n-button :disabled="!hasImage" @click="rotate('left')">左转</n-button>
      <n-button :disabled="!hasImage" @click="rotate('right')">右转</n-button>

      <div class="h-5 w-px bg-gray-200 mx-1" />

      <n-button type="success" :disabled="!hasImage" @click="exportCrop">导出</n-button>
    </div>

    <!-- 设置区 -->
    <div class="rounded border bg-white p-3 flex flex-col gap-3">
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-3">
        <div class="flex items-center justify-between gap-2">
          <span class="text-sm text-gray-600">导出格式</span>
          <n-select
            v-model:value="config.outputType"
            size="small"
            class="w-28"
            :options="outputTypeOptions"
          />
        </div>

        <div class="flex items-center justify-between gap-2">
          <span class="text-sm text-gray-600">固定比例</span>
          <n-switch v-model:value="config.fixed" size="small" />
        </div>

        <div class="flex items-center justify-between gap-2">
          <span class="text-sm text-gray-600">限制裁剪框在图内</span>
          <n-switch v-model:value="config.centerBox" size="small" />
        </div>

        <div class="flex items-center justify-between gap-2">
          <span class="text-sm text-gray-600">质量</span>
          <span class="text-sm font-medium text-gray-800">最高</span>
        </div>
      </div>

      <n-alert type="info" :show-icon="false">
        “输出尺寸”用于锁定裁剪框比例（宽:高）。如需让裁剪框立刻变成该尺寸，点“应用到裁剪框”。
      </n-alert>

      <div class="flex flex-wrap items-center gap-2">
        <div class="grid grid-cols-2 gap-2">
          <n-input-number v-model:value="config.outW" :min="1" :max="10000" size="small">
            <template #prefix>宽</template>
          </n-input-number>
          <n-input-number v-model:value="config.outH" :min="1" :max="10000" size="small">
            <template #prefix>高</template>
          </n-input-number>
        </div>

        <n-button size="small" @click="applySizeToCropBox" :disabled="!hasImage">应用到裁剪框</n-button>
        <n-button size="small" @click="setPreset(1, 1)" :disabled="!hasImage">1:1</n-button>
        <n-button size="small" @click="setPreset(4, 3)" :disabled="!hasImage">4:3</n-button>
        <n-button size="small" @click="setPreset(16, 9)" :disabled="!hasImage">16:9</n-button>
      </div>

      <div class="flex flex-wrap items-center gap-3 text-xs text-gray-500">
        <div class="truncate" v-if="filePath">文件：{{ filePath }}</div>
        <div v-if="imageInfo">尺寸：{{ imageInfo.width }} × {{ imageInfo.height }}（{{ imageInfo.format }}）</div>
        <div v-if="cropperInfo">当前输出：{{ cropperInfo.w }} × {{ cropperInfo.h }}</div>
        <div v-if="savePath" class="truncate">导出到：{{ savePath }}</div>
      </div>
    </div>

    <!-- 主要区域 -->
    <div class="flex-1 min-h-0 flex gap-3">
      <!-- 裁剪区 -->
      <div class="flex-1 min-w-0 rounded border bg-gray-50 overflow-hidden">
        <div v-if="!hasImage" class="h-full flex items-center justify-center text-gray-400">
          请选择一张图片开始裁剪
        </div>

        <vue-cropper
          v-else
          ref="cropperRef"
          class="w-full h-full"
          :img="imgUrl"
          :outputSize="config.outputSize"
          :outputType="config.outputType"
          :autoCrop="true"
          :autoCropWidth="config.cropBoxW"
          :autoCropHeight="config.cropBoxH"
          :fixed="config.fixed"
          :fixedNumber="[config.outW, config.outH]"
          :centerBox="config.centerBox"
          :canMove="true"
          :canMoveBox="true"
          :canScale="true"
          :info="true"
          :infoTrue="true"
          :maxImgSize="config.maxImgSize"
          :enlarge="config.enlarge"
          :mode="'contain'"
          @imgLoad="onImgLoad"
        />
      </div>

    </div>

    <div v-if="errorMsg" class="text-red-500 text-sm">{{ errorMsg }}</div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, reactive, ref, watch } from 'vue'
import { useMessage, NButton, NInputNumber, NSwitch, NAlert, NSelect } from 'naive-ui'
import { open, save } from '@tauri-apps/plugin-dialog'
import { readFile, writeFile } from '@tauri-apps/plugin-fs'
import { invoke } from '@tauri-apps/api/core'
import { VueCropper } from 'vue-cropper'
import 'vue-cropper/dist/index.css'

type OutputType = 'png' | 'jpeg' | 'webp'

const message = useMessage()

const cropperRef = ref<any>(null)
const imgUrl = ref<string | null>(null)
const filePath = ref<string | null>(null)
const savePath = ref<string | null>(null)
const errorMsg = ref<string>('')

const imageInfo = ref<{ width: number; height: number; format: string } | null>(null)
const cropperInfo = ref<{ w: number; h: number } | null>(null)

const outputTypeOptions = [
  { label: 'PNG', value: 'png' },
  { label: 'JPEG', value: 'jpeg' },
  { label: 'WEBP', value: 'webp' }
]

const config = reactive({
  // “输出尺寸”用于固定比例（宽:高）
  outW: 1,
  outH: 1,
  fixed: false,

  // 裁剪框默认大小（只影响初始生成/重置/应用）
  cropBoxW: 400,
  cropBoxH: 400,

  // 导出参数
  outputType: 'png' as OutputType,
  outputSize: 1,
  enlarge: 1,

  // 大图处理：越大越清晰，但更吃内存（vue-cropper 默认 2000）
  maxImgSize: 8000,

  // 限制裁剪框必须在图内（建议打开，可避免边界 bug）
  centerBox: true
})

const hasImage = computed(() => !!imgUrl.value)

function getFileName(p: string) {
  const i = Math.max(p.lastIndexOf('/'), p.lastIndexOf('\\'))
  return i >= 0 ? p.slice(i + 1) : p
}

function stripExt(name: string) {
  const idx = name.lastIndexOf('.')
  return idx > 0 ? name.slice(0, idx) : name
}

function dirOf(p: string) {
  const i = Math.max(p.lastIndexOf('/'), p.lastIndexOf('\\'))
  return i >= 0 ? p.slice(0, i) : ''
}

function joinPath(dir: string, name: string) {
  if (!dir) return name
  const sep = dir.includes('\\') ? '\\' : '/'
  return dir.endsWith(sep) ? dir + name : dir + sep + name
}

function extForOutputType(t: OutputType) {
  if (t === 'jpeg') return 'jpg'
  return t
}

function ensureExt(p: string, outputType: OutputType) {
  const ext = extForOutputType(outputType)
  const lower = p.toLowerCase()
  if (lower.endsWith(`.${ext}`)) return p
  // 如果用户手动写了别的扩展名，就尊重用户，不强行改
  if (/\.[a-z0-9]+$/i.test(p)) return p
  return `${p}.${ext}`
}

async function pickImage() {
  errorMsg.value = ''
  const selected = await open({
    multiple: false,
    filters: [
      {
        name: 'Images',
        extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'gif', 'tiff']
      }
    ]
  })

  if (!selected || Array.isArray(selected)) return

  filePath.value = selected

  // 读文件 -> blob url
  const bytes = await readFile(selected)
  const blob = new Blob([bytes])
  const url = URL.createObjectURL(blob)
  imgUrl.value = url

  // 读取图片信息（你已有的 Rust 命令）
  try {
    const info = await invoke<{ width: number; height: number; format: string }>('get_image_info', {
      path: selected
    })
    imageInfo.value = info
  } catch {
    imageInfo.value = null
  }

  // 默认输出比例 = 1:1
  config.outW = 1
  config.outH = 1

  // 默认裁剪框大小（更合理：根据窗口给一个中等值，后续可手动“应用”）
  config.cropBoxW = 420
  config.cropBoxH = 420

  savePath.value = null

  await nextTick()
  cropperRef.value?.goAutoCrop?.()
}

function onImgLoad(status: 'success' | 'error') {
  if (status !== 'success') {
    errorMsg.value = '图片加载失败，请检查文件是否损坏或格式不支持'
    return
  }
  errorMsg.value = ''
  updateCropperInfo()
}

function updateCropperInfo() {
  if (!cropperRef.value) return
  cropperInfo.value = {
    w: Math.round(cropperRef.value.cropW || 0),
    h: Math.round(cropperRef.value.cropH || 0)
  }
}

function zoom(delta: number) {
  cropperRef.value?.changeScale?.(delta)
  updateCropperInfo()
}

function rotate(dir: 'left' | 'right') {
  if (dir === 'left') cropperRef.value?.rotateLeft?.()
  else cropperRef.value?.rotateRight?.()
  updateCropperInfo()
}

function resetCrop() {
  cropperRef.value?.clearCrop?.()
  cropperRef.value?.refresh?.()
  nextTick(() => cropperRef.value?.goAutoCrop?.())
  updateCropperInfo()
}

function applySizeToCropBox() {
  // 把当前“输出尺寸”映射到“裁剪框默认大小”，然后重新生成裁剪框
  config.cropBoxW = Math.max(10, Math.min(2000, Math.round(config.outW)))
  config.cropBoxH = Math.max(10, Math.min(2000, Math.round(config.outH)))
  nextTick(() => cropperRef.value?.goAutoCrop?.())
}

function setPreset(w: number, h: number) {
  config.outW = w
  config.outH = h
  // 预设通常希望立刻作用到裁剪框
  applySizeToCropBox()
}

async function ensureSavePath() {
  if (savePath.value) return true
  if (!filePath.value) return false

  const fileName = getFileName(filePath.value)
  const baseName = stripExt(fileName)
  const ext = extForOutputType(config.outputType)
  const defaultPath = joinPath(dirOf(filePath.value), `${baseName}-crop.${ext}`)

  const picked = await save({
    defaultPath,
    filters: [{ name: config.outputType.toUpperCase(), extensions: [ext] }]
  })
  if (!picked) return false

  savePath.value = ensureExt(picked, config.outputType)
  return true
}

function getCropBlob(): Promise<Blob> {
  return new Promise((resolve, reject) => {
    try {
      cropperRef.value?.getCropBlob?.((blob: Blob) => {
        if (!blob) reject(new Error('getCropBlob 返回空数据'))
        else resolve(blob)
      })
    } catch (e) {
      reject(e)
    }
  })
}

/**
 * ✅ 关键修复：
 * 之前用 getCropAxis + Rust 裁剪，会出现“导出和预览不一致”的问题，
 * 因为 getCropAxis 返回的是“容器坐标”，会受 contain 模式、边距、缩放影响。
 *
 * 现在导出直接使用 vue-cropper 的 getCropBlob（和预览同一套渲染管线），
 * 导出的内容与当前看到的裁剪结果一致。
 */
async function exportCrop() {
  try {
    errorMsg.value = ''

    if (!hasImage.value) {
      message.warning('请先选择图片')
      return
    }
    const ok = await ensureSavePath()
    if (!ok || !savePath.value) return

    const blob = await getCropBlob()
    const bytes = new Uint8Array(await blob.arrayBuffer())

    await writeFile(savePath.value, bytes)

    message.success('导出成功')
  } catch (err: any) {
    // 如果这里报 forbidden path，大概率是 fs scope 没放开
    errorMsg.value = err?.message || String(err) || '导出失败'
  }
}

// 释放 ObjectURL，避免内存泄漏
watch(imgUrl, (next, prev) => {
  if (prev) URL.revokeObjectURL(prev)
})

// 轻量同步裁剪框信息（拖动/缩放时 cropW/cropH 会变）
let infoTimer: number | null = null
watch(
  () => [config.outW, config.outH, config.fixed, config.centerBox, config.outputType, config.outputSize, config.maxImgSize, config.enlarge],
  () => {
    // 防抖一下，避免输入时频繁触发
    if (infoTimer) window.clearTimeout(infoTimer)
    infoTimer = window.setTimeout(() => updateCropperInfo(), 80)
  },
  { deep: true }
)

onBeforeUnmount(() => {
  if (imgUrl.value) URL.revokeObjectURL(imgUrl.value)
  if (infoTimer) window.clearTimeout(infoTimer)
})
</script>

<style scoped>
/* 让 vue-cropper 撑满容器 */
:deep(.cropper-box) {
  height: 100%;
}
</style>
