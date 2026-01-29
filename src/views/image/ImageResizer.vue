<script setup lang="ts">
import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'
import { NInputNumber, NButton, NIcon, NRadioGroup, NRadioButton, useMessage } from 'naive-ui'
import { Image, Save, Locked, Unlocked } from '@vicons/carbon'

const message = useMessage()
const currentFile = ref<{ path: string; name: string } | null>(null)
const originalSize = ref<{ w: number; h: number }>({ w: 0, h: 0 })

// 模式：pixel (像素) | percent (百分比)
const mode = ref<'pixel' | 'percent'>('pixel')
// 是否锁定纵横比
const lockRatio = ref(true)

// 输入值
const targetW = ref(0)
const targetH = ref(0)
const percent = ref(100)

// 选择文件
const selectFile = async () => {
  const selected = await open({
    multiple: false,
    filters: [{ name: 'Image', extensions: ['png', 'jpg', 'jpeg', 'webp'] }]
  })

  if (selected) {
    const name = selected.split(/[\\/]/).pop() || 'image'
    currentFile.value = { path: selected, name }

    // 调用 Rust 获取原始尺寸
    try {
      const [w, h] = await invoke<[number, number]>('get_image_info', { path: selected })
      originalSize.value = { w, h }
      // 初始化输入框
      targetW.value = w
      targetH.value = h
      percent.value = 100
    } catch (e: any) {
      message.error(e)
    }
  }
}

// 监听像素变化
const onWidthChange = (val: number | null) => {
  if (!val) return
  targetW.value = val
  if (lockRatio.value && originalSize.value.w > 0) {
    // 宽变了，自动算高： 高 = 宽 / 原比例
    const ratio = originalSize.value.w / originalSize.value.h
    targetH.value = Math.round(val / ratio)
  }
}

const onHeightChange = (val: number | null) => {
  if (!val) return
  targetH.value = val
  if (lockRatio.value && originalSize.value.h > 0) {
    // 高变了，自动算宽
    const ratio = originalSize.value.w / originalSize.value.h
    targetW.value = Math.round(val * ratio)
  }
}

// 监听百分比变化
watch(percent, (val) => {
  if (mode.value === 'percent') {
    targetW.value = Math.round(originalSize.value.w * (val / 100))
    targetH.value = Math.round(originalSize.value.h * (val / 100))
  }
})

// 切换模式时的同步
watch(mode, (newMode) => {
  if (newMode === 'percent') {
    // 切到百分比，反推当前是百分之多少（以宽为准）
    if (originalSize.value.w > 0) {
      percent.value = Math.round((targetW.value / originalSize.value.w) * 100)
    }
  } else {
    // 切到像素，数值已经在那了，不用动
  }
})

// 执行保存
const handleSave = async () => {
  if (!currentFile.value) return

  // 1. 选择保存路径
  const savePath = await save({
    defaultPath: currentFile.value.path.replace(/(\.[\w]+)$/, '_resized$1'),
    filters: [{ name: 'Image', extensions: ['png', 'jpg', 'jpeg', 'webp'] }]
  })

  if (!savePath) return

  // 2. 调用 Rust
  try {
    await invoke('resize_image', {
      inputPath: currentFile.value.path,
      outputPath: savePath,
      width: targetW.value,
      height: targetH.value
    })
    message.success(`修改成功！尺寸: ${targetW.value}x${targetH.value}`)
  } catch (e: any) {
    message.error('修改失败: ' + e)
  }
}
</script>

<template>
  <div class="max-w-3xl mx-auto py-8 px-4">
    <div class="mb-8">
      <h2 class="text-2xl font-bold text-slate-100">图片尺寸调整</h2>
      <p class="text-slate-400 text-sm">高性能 · 高质量 Lanczos3 算法</p>
    </div>

    <div class="bg-[#1E293B] border border-slate-700/50 rounded-2xl p-8 shadow-xl">
      <div v-if="!currentFile"
           @click="selectFile"
           class="h-40 border-2 border-dashed border-slate-600 rounded-xl flex flex-col items-center justify-center cursor-pointer hover:border-cyan-500 hover:bg-slate-800 transition-all group">
        <n-icon :size="40" :component="Image" class="text-slate-500 group-hover:text-cyan-400 mb-2 transition-colors" />
        <span class="text-slate-400 group-hover:text-slate-200">点击选择图片</span>
      </div>

      <div v-else>
        <div class="flex items-center justify-between bg-slate-800 p-4 rounded-xl mb-6 border border-slate-700">
          <div class="flex items-center gap-4">
            <div class="w-12 h-12 bg-cyan-500/20 rounded-lg flex items-center justify-center text-cyan-400">
              <n-icon :size="24" :component="Image" />
            </div>
            <div>
              <div class="text-slate-200 font-medium truncate max-w-[200px]">{{ currentFile.name }}</div>
              <div class="text-xs text-slate-500">原始尺寸: {{ originalSize.w }} x {{ originalSize.h }} px</div>
            </div>
          </div>
          <n-button size="small" secondary @click="currentFile = null">重新选择</n-button>
        </div>

        <div class="space-y-6">
          <div class="flex items-center gap-4">
            <span class="text-slate-400 w-20">调整方式</span>
            <n-radio-group v-model:value="mode" name="mode">
              <n-radio-button value="pixel" label="按像素" />
              <n-radio-button value="percent" label="按百分比" />
            </n-radio-group>
          </div>

          <div v-if="mode === 'pixel'" class="flex items-start gap-4 animate-fade-in">
            <div class="flex-1">
              <div class="text-xs text-slate-500 mb-1">宽度 (px)</div>
              <n-input-number v-model:value="targetW" :min="1" @update:value="onWidthChange" />
            </div>

            <div class="pt-6 flex flex-col items-center gap-1 cursor-pointer" @click="lockRatio = !lockRatio">
              <div class="p-2 rounded hover:bg-slate-700 transition-colors" :class="lockRatio ? 'text-cyan-400' : 'text-slate-600'">
                <n-icon :size="20" :component="lockRatio ? Locked : Unlocked" />
              </div>
            </div>

            <div class="flex-1">
              <div class="text-xs text-slate-500 mb-1">高度 (px)</div>
              <n-input-number v-model:value="targetH" :min="1" @update:value="onHeightChange" />
            </div>
          </div>

          <div v-else class="animate-fade-in">
            <div class="flex items-center gap-4">
              <span class="text-slate-400 w-20">缩放比例</span>
              <div class="flex-1">
                <n-input-number v-model:value="percent" :min="1" :max="500">
                  <template #suffix>%</template>
                </n-input-number>
              </div>
            </div>
            <div class="mt-2 text-right text-xs text-slate-500">
              预期结果: {{ Math.round(originalSize.w * percent / 100) }} x {{ Math.round(originalSize.h * percent / 100) }}
            </div>
          </div>

          <div class="pt-4 border-t border-slate-700 mt-6">
            <n-button type="primary" size="large" block @click="handleSave" class="font-bold">
              <template #icon><n-icon :component="Save" /></template>
              保存调整后的图片
            </n-button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.animate-fade-in {
  animation: fadeIn 0.3s ease-in-out;
}
@keyframes fadeIn {
  from { opacity: 0; transform: translateY(-5px); }
  to { opacity: 1; transform: translateY(0); }
}
</style>