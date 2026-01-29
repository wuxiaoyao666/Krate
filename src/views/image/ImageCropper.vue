<script setup lang="ts">
import { ref, reactive, onUnmounted } from 'vue'
import 'vue-cropper/dist/index.css'
import { VueCropper } from 'vue-cropper'

import { invoke } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'
import { readFile } from '@tauri-apps/plugin-fs'
import { NInputNumber, NButton, NIcon, useMessage, NSwitch } from 'naive-ui'
import { Image, Save, Crop } from '@vicons/carbon'

const message = useMessage()
const cropperRef = ref()

// 状态
const imageUrl = ref('')
const rawPath = ref('')
const fileName = ref('')

// 存储原图真实尺寸 (从 Rust 获取)
const realSize = reactive({ w: 0, h: 0 })

// 裁剪配置
const config = reactive({
  width: 400,
  height: 400,
  fixedBox: false,
  fixed: true,
})

const selectFile = async () => {
  try {
    const selected = await open({
      multiple: false,
      filters: [{ name: 'Image', extensions: ['png', 'jpg', 'jpeg', 'webp'] }]
    })

    if (selected) {
      rawPath.value = selected
      fileName.value = selected.split(/[\\/]/).pop() || 'image'

      // 1. 先读取文件显示
      const fileBytes = await readFile(selected)
      const blob = new Blob([fileBytes])
      const url = URL.createObjectURL(blob)
      if (imageUrl.value) URL.revokeObjectURL(imageUrl.value)
      imageUrl.value = url

      try {
        const [w, h] = await invoke<[number, number]>('get_image_info', { path: selected })
        realSize.w = w
        realSize.h = h
        console.log(`原图真实尺寸: ${w}x${h}`)
      } catch (e) {
        console.error('获取图片信息失败', e)
      }
    }
  } catch (e: any) {
    message.error('读取图片失败: ' + e)
  }
}

onUnmounted(() => {
  if (imageUrl.value) URL.revokeObjectURL(imageUrl.value)
})

// 保存逻辑
const handleSave = async () => {
  if (!cropperRef.value) return

  const axis = cropperRef.value.getCropAxis()

  // 打印看看结构 (通常包含 x1, y1, x2, y2)
  console.log('获取到的原始坐标:', axis)

  if (!axis) {
    message.error('无法获取裁剪区域')
    return
  }

  // 我们需要转换成 Rust 需要的 x, y, width, height
  const x = Math.round(Math.max(0, axis.x1))
  const y = Math.round(Math.max(0, axis.y1))

  // 宽 = 右边界 - 左边界
  const w = Math.round(axis.x2 - axis.x1)
  // 高 = 下边界 - 上边界
  const h = Math.round(axis.y2 - axis.y1)

  console.log(`计算结果: x=${x}, y=${y}, w=${w}, h=${h}`)

  if (w <= 0 || h <= 0) {
    message.error('裁剪区域无效')
    return
  }

  // 3. 选择保存路径
  const savePath = await save({
    defaultPath: rawPath.value.replace(/(\.[\w]+)$/, '_crop$1'),
    filters: [{ name: 'Image', extensions: ['png', 'jpg', 'jpeg'] }]
  })

  if (!savePath) return

  // 4. 调用 Rust
  try {
    await invoke('crop_image', {
      inputPath: rawPath.value,
      outputPath: savePath,
      x, y, width: w, height: h
    })
    message.success(`裁剪成功！`)
  } catch (e: any) {
    const err = typeof e === 'string' ? e : e.message
    message.error('裁剪失败: ' + err)
  }
}

const setRatio = (w: number, h: number) => {
  config.width = w
  config.height = h
  config.fixed = true
  // 强制刷新组件状态
  setTimeout(() => {
    if(cropperRef.value) cropperRef.value.refresh()
  }, 10)
}
</script>

<template>
  <div class="h-full flex flex-col p-4 gap-4">
    <div class="flex justify-between items-center bg-[#1E293B] p-4 rounded-xl border border-slate-700">
      <div class="flex items-center gap-4">
        <n-button @click="selectFile" type="primary" secondary>
          <template #icon><n-icon :component="Image" /></template>
          打开图片
        </n-button>
        <div v-if="fileName" class="text-slate-300 text-sm">
          当前编辑: <span class="text-cyan-400 font-bold">{{ fileName }}</span>
          <span class="ml-2 text-slate-500 text-xs" v-if="realSize.w">({{ realSize.w }}x{{ realSize.h }})</span>
        </div>
      </div>

      <n-button type="primary" @click="handleSave" :disabled="!imageUrl">
        <template #icon><n-icon :component="Save" /></template>
        保存裁剪结果
      </n-button>
    </div>

    <div class="flex-1 flex gap-4 min-h-0">
      <div class="flex-1 bg-black/50 rounded-xl overflow-hidden relative border border-slate-700">
        <div v-if="!imageUrl" class="absolute inset-0 flex items-center justify-center text-slate-500">
          请先打开图片
        </div>

        <vue-cropper
          v-if="imageUrl"
          ref="cropperRef"
          :img="imageUrl"
          :outputSize="1"
          outputType="png"
          :autoCrop="true"
          :autoCropWidth="config.width"
          :autoCropHeight="config.height"
          :fixed="config.fixed"
          :fixedNumber="[config.width, config.height]"
          :centerBox="false"
          :full="true"
          mode="contain"
        ></vue-cropper>
      </div>

      <div class="w-72 bg-[#1E293B] p-4 rounded-xl border border-slate-700 flex flex-col gap-6">
        <div>
          <h3 class="text-slate-200 font-bold mb-4 flex items-center gap-2">
            <n-icon :component="Crop" /> 裁剪设置
          </h3>

          <div class="space-y-4">
            <div>
              <div class="text-xs text-slate-400 mb-1">目标宽度 (px)</div>
              <n-input-number v-model:value="config.width" :min="1" />
            </div>
            <div>
              <div class="text-xs text-slate-400 mb-1">目标高度 (px)</div>
              <n-input-number v-model:value="config.height" :min="1" />
            </div>

            <div class="flex items-center justify-between pt-2">
              <span class="text-sm text-slate-400">锁定比例</span>
              <n-switch v-model:value="config.fixed" size="small" />
            </div>
            <div class="text-xs text-slate-500">
              锁定后，拖拽边框会保持 {{config.width}}:{{config.height}} 的比例
            </div>
          </div>
        </div>

        <div class="border-t border-slate-700 pt-4">
          <h4 class="text-slate-300 text-sm mb-2">常用比例预设</h4>
          <div class="grid grid-cols-2 gap-2">
            <n-button size="small" secondary @click="setRatio(1080, 1920)">1080p (竖)</n-button>
            <n-button size="small" secondary @click="setRatio(1920, 1080)">1080p (横)</n-button>
            <n-button size="small" secondary @click="setRatio(800, 800)">1:1 方形</n-button>
            <n-button size="small" secondary @click="setRatio(1280, 720)">720p</n-button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style>
</style>