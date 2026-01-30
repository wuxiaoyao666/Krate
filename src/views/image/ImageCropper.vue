<script setup lang="ts">
import { ref, reactive, onUnmounted, nextTick, computed } from 'vue'
import 'vue-cropper/dist/index.css'
import { VueCropper } from 'vue-cropper'

import { open, save } from '@tauri-apps/plugin-dialog'
import { readFile, writeFile } from '@tauri-apps/plugin-fs'
import {
  NButton,
  NIcon,
  useMessage,
  NSelect,
  NSwitch,
  NTooltip,
  NInputNumber
} from 'naive-ui'
import {
  Image as ImageIcon,
  Save,
  Reset,
  ZoomIn,
  ZoomOut,
  RotateClockwise,
  RotateCounterclockwise,
  Crop
} from '@vicons/carbon'

const message = useMessage()
const cropperRef = ref<any>(null)

// ================= 状态 =================
const imageUrl = ref('')
const rawPath = ref('')
const fileName = ref('')
const loading = ref(false)

// 原图尺寸
const realSize = reactive({ w: 0, h: 0 })

// 当前裁剪输出尺寸
const cropInfo = reactive({ w: 0, h: 0 })

// windows 比例可能不是 100%
const dpr = window.devicePixelRatio || 1

// 配置项
const config = reactive({
  outputFormat: 'png' as 'png' | 'jpeg' | 'webp',
  lockRatio: false,
  ratioW: 1,
  ratioH: 1,
  centerBox: true
})

const formatOptions = [
  { label: 'PNG', value: 'png' },
  { label: 'JPG', value: 'jpeg' },
  { label: 'WEBP', value: 'webp' }
]

const outputExt = computed(() =>
  config.outputFormat === 'jpeg' ? 'jpg' : config.outputFormat
)

// ================= 工具函数 =================
const loadImageNaturalSize = (url: string) =>
  new Promise<void>((resolve) => {
    const img = new Image()
    img.onload = () => {
      realSize.w = img.naturalWidth
      realSize.h = img.naturalHeight
      resolve()
    }
    img.onerror = () => resolve()
    img.src = url
  })

const updateCropInfo = (data?: any) => {
  if (data?.w && data?.h) {
    // realTime 的 w/h 是 CSS px；导出 canvas 通常按 DPR 放大
    cropInfo.w = Math.round(data.w * dpr)
    cropInfo.h = Math.round(data.h * dpr)
  }
}

const getCropBlob = async (): Promise<Blob> => {
  const inst = cropperRef.value
  if (!inst?.getCropBlob) throw new Error('裁剪组件未就绪')

  return new Promise((resolve, reject) => {
    inst.getCropBlob((blob: Blob) => {
      if (blob) resolve(blob)
      else reject(new Error('获取裁剪结果失败'))
    })
  })
}

// ================= 核心逻辑 =================
const selectFile = async () => {
  try {
    const selected = await open({
      multiple: false,
      filters: [{ name: 'Image', extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp'] }]
    })
    if (!selected) return

    loading.value = true
    rawPath.value = selected
    fileName.value = selected.split(/[\\/]/).pop() || 'image'

    const bytes = await readFile(selected)
    const blob = new Blob([bytes])
    const url = URL.createObjectURL(blob)

    if (imageUrl.value) URL.revokeObjectURL(imageUrl.value)
    imageUrl.value = url

    await loadImageNaturalSize(url)
    await nextTick()
    cropperRef.value?.goAutoCrop?.()
  } catch (e: any) {
    message.error('读取失败: ' + (e?.message ?? e))
  } finally {
    loading.value = false
  }
}

const handleSave = async () => {
  if (!imageUrl.value) return

  try {
    const ext = outputExt.value
    const defaultName = rawPath.value
      ? rawPath.value.replace(/\.[^.]+$/, `_crop.${ext}`)
      : `crop.${ext}`

    const savePath = await save({
      defaultPath: defaultName,
      filters: [{ name: ext.toUpperCase(), extensions: [ext] }]
    })
    if (!savePath) return

    const toast = message.loading('正在导出...', { duration: 0 })
    try {
      const blob = await getCropBlob()
      const buffer = new Uint8Array(await blob.arrayBuffer())

      await writeFile(savePath, buffer)

      message.success('导出成功')
    } finally {
      toast.destroy()
    }
  } catch (e: any) {
    console.error(e)
    message.error('导出失败: ' + (e?.message ?? e))
  }
}


const zoom = (v: number) => cropperRef.value?.changeScale?.(v)
const rotate = (d: 'left' | 'right') =>
  d === 'left'
    ? cropperRef.value?.rotateLeft?.()
    : cropperRef.value?.rotateRight?.()

const reset = () => {
  cropperRef.value?.refresh?.()
  nextTick(() => cropperRef.value?.goAutoCrop?.())
}

const onRatioChanged = async () => {
  if (!config.lockRatio) return
  await nextTick()
  cropperRef.value?.refresh?.()
  nextTick(() => cropperRef.value?.goAutoCrop?.())
}

onUnmounted(() => {
  if (imageUrl.value) URL.revokeObjectURL(imageUrl.value)
})
</script>

<template>
  <div class="h-full flex flex-col bg-[#0F172A] p-4 gap-4">
    <!-- 顶部工具栏（两行） -->
    <div class="bg-slate-800/80 p-3 rounded-lg border border-slate-700 backdrop-blur flex flex-col gap-3">
      <!-- 第一行：文件 / 导出 -->
      <div class="flex items-center gap-4">
        <NButton type="primary" size="medium" :loading="loading" @click="selectFile">
          <template #icon><NIcon><ImageIcon /></NIcon></template>
          选择图片
        </NButton>

        <div v-if="fileName" class="flex flex-col leading-tight min-w-0">
          <span class="text-slate-200 font-bold text-sm truncate max-w-[200px]">
            {{ fileName }}
          </span>
          <span class="text-slate-500 text-xs font-mono">
            原图: {{ realSize.w }} × {{ realSize.h }}
          </span>
        </div>

        <div class="flex-1"></div>

        <div v-if="imageUrl" class="flex items-center gap-3">
          <span class="text-slate-400 text-xs">格式</span>
          <NSelect
            v-model:value="config.outputFormat"
            :options="formatOptions"
            size="small"
            class="w-[110px]"
          />
          <NButton type="success" size="medium" @click="handleSave">
            <template #icon><NIcon><Save /></NIcon></template>
            导出
          </NButton>
        </div>
      </div>

      <!-- 第二行：裁剪编辑 -->
      <div v-if="imageUrl" class="flex flex-wrap items-center gap-4">
        <div class="flex items-center gap-2">
          <span class="text-slate-400 text-xs">固定比例</span>
          <NSwitch v-model:value="config.lockRatio" size="small" />
        </div>

        <div v-if="config.lockRatio" class="flex items-center gap-2">
          <span class="text-slate-400 text-xs">W:H</span>
          <NInputNumber
            v-model:value="config.ratioW"
            :min="1"
            size="tiny"
            class="w-20"
            @update:value="onRatioChanged"
          />
          <span class="text-slate-500 text-xs">:</span>
          <NInputNumber
            v-model:value="config.ratioH"
            :min="1"
            size="tiny"
            class="w-20"
            @update:value="onRatioChanged"
          />
        </div>

        <div class="w-px h-4 bg-slate-700"></div>

        <div class="flex items-center gap-2">
          <span class="text-slate-400 text-xs">框在图内</span>
          <NSwitch v-model:value="config.centerBox" size="small" />
        </div>

        <div class="w-px h-4 bg-slate-700"></div>

        <NTooltip>
          <template #trigger>
            <NButton strong secondary circle size="small" @click="rotate('left')">
              <template #icon><NIcon><RotateCounterclockwise /></NIcon></template>
            </NButton>
          </template>
          向左旋转
        </NTooltip>

        <NTooltip>
          <template #trigger>
            <NButton strong secondary circle size="small" @click="rotate('right')">
              <template #icon><NIcon><RotateClockwise /></NIcon></template>
            </NButton>
          </template>
          向右旋转
        </NTooltip>

        <NTooltip>
          <template #trigger>
            <NButton strong secondary circle size="small" @click="zoom(0.2)">
              <template #icon><NIcon><ZoomIn /></NIcon></template>
            </NButton>
          </template>
          放大
        </NTooltip>

        <NTooltip>
          <template #trigger>
            <NButton strong secondary circle size="small" @click="zoom(-0.2)">
              <template #icon><NIcon><ZoomOut /></NIcon></template>
            </NButton>
          </template>
          缩小
        </NTooltip>

        <NButton strong secondary size="small" @click="reset">
          <template #icon><NIcon><Reset /></NIcon></template>
          重置
        </NButton>
      </div>
    </div>

    <!-- 主画布 -->
    <div class="flex-1 bg-slate-900/50 rounded-lg overflow-hidden border border-slate-700 relative shadow-inner">
      <div
        v-if="!imageUrl"
        class="absolute inset-0 flex flex-col items-center justify-center text-slate-500 gap-3"
      >
        <NIcon size="48" :component="Crop" class="opacity-40" />
        <span class="text-sm">请选择一张图片</span>
      </div>

      <VueCropper
        v-if="imageUrl"
        ref="cropperRef"
        :img="imageUrl"
        :outputSize="1"
        :outputType="config.outputFormat"
        :fixed="config.lockRatio"
        :fixedNumber="[config.ratioW, config.ratioH]"
        :autoCrop="true"
        :centerBox="config.centerBox"
        :canMove="true"
        :canMoveBox="true"
        :high="true"
        mode="contain"
        @realTime="updateCropInfo"
      />

      <div
        v-if="imageUrl"
        class="absolute bottom-4 right-4 bg-black/70 text-white px-3 py-1.5 rounded text-xs font-mono"
      >
        输出尺寸:
        <span class="text-emerald-400 font-bold ml-1">
          {{ cropInfo.w }} × {{ cropInfo.h }}
        </span>
        px
      </div>
    </div>
  </div>
</template>
