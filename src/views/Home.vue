<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { routes } from '@/router/routes'
import {
  NInput, NIcon, NGrid, NGridItem, NCard, NText
} from 'naive-ui'

import { Search, SettingsAdjust, UserAvatar } from '@vicons/carbon'

const router = useRouter()
const searchText = ref('')

const dashboardGroups = computed(() => {
  const visibleRoutes = routes.filter(r => !r.meta?.hidden && r.path !== '/')

  return visibleRoutes.map(group => {
    if (group.children && group.children.length > 0) {
      return {
        name: group.meta?.title || group.name,
        items: group.children
          .filter(child => !child.meta?.hidden)
          .map(child => ({
            key: child.name,
            label: child.meta?.title,
            icon: child.meta?.icon || UserAvatar,
            desc: child.meta?.desc || child.meta?.title
          }))
      }
    }
    return {
      name: group.meta?.title || '其他',
      items: [{
        key: group.name,
        label: group.meta?.title,
        icon: group.meta?.icon || UserAvatar,
        desc: group.meta?.desc || '快捷入口'
      }]
    }
  })
})

const filteredGroups = computed(() => {
  if (!searchText.value) return dashboardGroups.value
  const lower = searchText.value.toLowerCase()
  return dashboardGroups.value.map(group => ({
    ...group,
    items: group.items.filter((item: any) =>
      String(item.label).toLowerCase().includes(lower) ||
      String(item.desc).toLowerCase().includes(lower)
    )
  })).filter(group => group.items.length > 0)
})

const handleJump = (name: string | undefined) => {
  if (name) router.push({ name })
}

const greeting = computed(() => {
  const hour = new Date().getHours()
  if (hour < 6) return '夜深了，注意休息'
  if (hour < 12) return '早上好，开启高效的一天'
  if (hour < 14) return '午饭时间，记得休息'
  if (hour < 19) return '下午好，继续加油'
  return '晚上好，享受属于你的时间'
})
</script>

<template>
  <div class="page-container h-full flex flex-col p-8 bg-[#0F172A]">
    <div class="mb-8 animate-fade-in-down">
      <h1 class="text-3xl font-bold text-slate-100 mb-2">Krate 工具箱</h1>
      <p class="text-slate-400">{{ greeting }}。</p>
    </div>

    <div class="mb-10 max-w-2xl">
      <n-input
        v-model:value="searchText"
        size="large"
        placeholder="搜索工具"
        clearable
        class="search-input"
      >
        <template #prefix>
          <n-icon :component="Search" class="mr-2 text-slate-400" />
        </template>
      </n-input>
    </div>

    <div class="flex-1 overflow-y-auto custom-scrollbar pr-2">
      <div v-for="group in filteredGroups" :key="group.name as string" class="mb-8">
        <h3 class="text-lg font-semibold text-slate-300 mb-4 flex items-center">
          <span class="w-1 h-4 bg-emerald-500 rounded mr-2"></span>
          {{ group.name }}
        </h3>

        <n-grid x-gap="16" y-gap="16" cols="2 s:3 m:4 l:5" responsive="screen">
          <n-grid-item v-for="item in group.items" :key="item.key as string">
            <n-card
              hoverable
              class="cursor-pointer bg-slate-800/50 border border-slate-700/50 group hover:border-emerald-500/50 transition-all duration-200 hover:-translate-y-1"
              content-style="padding: 16px;"
              @click="handleJump(item.key as string)"
            >
              <div class="flex flex-col h-full">
                <div class="flex items-center justify-between mb-3">
                  <div class="p-2 rounded-lg bg-slate-700/50 text-emerald-400 group-hover:bg-emerald-500/10 group-hover:text-emerald-400 transition-colors">
                    <component :is="item.icon" class="w-5 h-5" />
                  </div>
                </div>
                <n-text class="font-bold text-slate-200 text-base mb-1">{{ item.label }}</n-text>
                <n-text class="text-xs text-slate-500 line-clamp-2 h-8">{{ item.desc }}</n-text>
              </div>
            </n-card>
          </n-grid-item>
        </n-grid>
      </div>

      <div v-if="filteredGroups.length === 0" class="text-center py-20 text-slate-500">
        <n-icon size="48" :component="SettingsAdjust" class="mb-4 opacity-50" />
        <p>没有找到相关工具</p>
      </div>

    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: #334155;
  border-radius: 3px;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: #475569;
}
:deep(.search-input) {
  background-color: rgba(30, 41, 59, 0.5) !important;
  border-color: rgba(51, 65, 85, 1) !important;
  color: #e2e8f0 !important;
}
:deep(.n-input:hover) {
  border-color: #10b981 !important;
}
:deep(.n-input.n-input--focus) {
  border-color: #10b981 !important;
  box-shadow: 0 0 0 2px rgba(16, 185, 129, 0.2) !important;
}
.animate-fade-in-down {
  animation: fadeInDown 0.5s ease-out;
}
@keyframes fadeInDown {
  from { opacity: 0; transform: translateY(-10px); }
  to { opacity: 1; transform: translateY(0); }
}
</style>