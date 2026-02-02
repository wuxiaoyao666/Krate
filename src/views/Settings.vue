<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart'
import { getVersion } from '@tauri-apps/api/app'
import { NCard, NSwitch, NList, NListItem, NIcon, useMessage } from 'naive-ui'
import { Settings, Power, Information } from '@vicons/carbon'

const message = useMessage()
const autoStart = ref(false)
const appVersion = ref('')
const loading = ref(false)

onMounted(async () => {
  // 1. 获取版本号
  try {
    appVersion.value = await getVersion()
  } catch (e) {
    appVersion.value = 'Dev Build'
  }

  // 2. 检查自启状态
  try {
    autoStart.value = await isEnabled()
  } catch (e) {
    console.error('Failed to check autostart status:', e)
  }
})

const handleAutoStartChange = async (value: boolean) => {
  loading.value = true
  try {
    if (value) {
      await enable()
      message.success('已开启开机自启')
    } else {
      await disable()
      message.success('已关闭开机自启')
    }
    autoStart.value = value
  } catch (e) {
    message.error('设置失败，可能受系统权限限制')
    console.error(e)
    // 回滚状态
    autoStart.value = !value
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="p-8 h-full flex flex-col bg-[#0F172A]">
    <div class="mb-8">
      <h1 class="text-3xl font-bold text-slate-100 flex items-center gap-3">
        <NIcon class="text-slate-400"><Settings /></NIcon>
        系统设置
      </h1>
      <p class="text-slate-400 mt-2">管理 Krate 的系统级行为</p>
    </div>

    <div class="max-w-2xl">
      <NCard class="bg-slate-800/50 border-slate-700/50 rounded-2xl">
        <NList class="bg-transparent">
          <NListItem>
            <div class="flex items-center justify-between p-2">
              <div class="flex items-center gap-4">
                <div class="p-2 bg-blue-500/20 rounded-lg text-blue-400">
                  <NIcon size="24"><Power /></NIcon>
                </div>
                <div>
                  <div class="text-base font-bold text-slate-200">开机自动启动</div>
                  <div class="text-xs text-slate-500">
                    随系统启动并最小化到托盘
                  </div>
                </div>
              </div>
              <NSwitch
                v-model:value="autoStart"
                :loading="loading"
                @update:value="handleAutoStartChange"
              />
            </div>
          </NListItem>

          <NListItem>
            <div class="flex items-center justify-between p-2">
              <div class="flex items-center gap-4">
                <div class="p-2 bg-slate-700/50 rounded-lg text-slate-400">
                  <NIcon size="24"><Information /></NIcon>
                </div>
                <div>
                  <div class="text-base font-bold text-slate-200">当前版本</div>
                  <div class="text-xs text-slate-500">
                    Krate v{{ appVersion }}
                  </div>
                </div>
              </div>
              <div class="text-xs text-emerald-500 bg-emerald-500/10 px-2 py-1 rounded">
                已是最新
              </div>
            </div>
          </NListItem>
        </NList>
      </NCard>
    </div>
  </div>
</template>