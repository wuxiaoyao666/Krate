<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { NMenu } from 'naive-ui'
import { useRouter, useRoute } from 'vue-router'
import { routes } from '@/router/routes.ts'

const router = useRouter()
const route = useRoute()

// 1. 动态生成菜单选项
// 递归函数：把 Vue Router 的 routes 转换成 Naive UI 的 menuOptions
const transformRoutesToMenu = (routes: any[]) => {
  return routes
    .filter((item) => !item.meta?.hidden) // 过滤掉不需要显示的路由
    .map((item) => {
      const menuItem: any = {
        label: item.meta?.title || item.name,
        key: item.name,
        icon: item.meta?.icon,
      }

      // 如果有子路由，递归处理
      if (item.children && item.children.length > 0) {
        menuItem.children = transformRoutesToMenu(item.children)
      }
      return menuItem
    })
}

const menuOptions = transformRoutesToMenu(routes)

// 选中项：直接计算属性绑定当前路由 name
const activeKey = computed(() => route.name as string)

// 展开项：控制哪些父菜单是打开的
const expandedKeys = ref<string[]>([])

// 监听路由变化，自动展开父级菜单
// route.matched 包含了当前路由匹配到的所有层级（父 -> 子）
watch(
  () => route.matched,
  (matched) => {
    // 获取当前路由路径上需要展开的所有 key
    const currentRouteKeys = matched.map((m) => m.name as string)

    // 遍历这些 key，如果它不在 expandedKeys 里，就把它加进去
    currentRouteKeys.forEach(key => {
      if (!expandedKeys.value.includes(key)) {
        expandedKeys.value.push(key)
      }
    })
  },
  { immediate: true },
)

// 处理菜单点击
const handleUpdateValue = (key: string) => {
  router.push({ name: key })
}

const handleUpdateExpandedKeys = (keys: string[]) => {
  expandedKeys.value = keys
}
</script>

<template>
  <div class="flex flex-col h-full bg-[#0F172A]">
    <div class="h-16 flex items-center px-6 border-b border-slate-800/50">
      <div class="w-8 h-8 bg-cyan-500/20 rounded-lg flex items-center justify-center mr-3">
        <span class="text-cyan-400 font-bold">K</span>
      </div>
      <span class="text-lg font-bold tracking-wider text-slate-100">Krate</span>
    </div>

    <div class="flex-1 py-4 overflow-y-auto custom-scrollbar">
      <n-menu
        :options="menuOptions"
        :value="activeKey"
        :expanded-keys="expandedKeys"
        @update:expanded-keys="handleUpdateExpandedKeys"
        @update:value="handleUpdateValue"
        :indent="24"
      />
    </div>

    <div class="p-4 border-t border-slate-800/50">
      <div class="text-xs text-slate-500 text-center">v1.0.0 Dev</div>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar {
  width: 0;
}
</style>
