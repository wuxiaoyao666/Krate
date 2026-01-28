<script setup lang="ts">
import { h, ref } from 'vue'
import { NMenu, NIcon } from 'naive-ui'
import { useRouter } from 'vue-router'
import type { MenuOption } from 'naive-ui'
import {
  Code as TextIcon,
  Document as DocIcon,
  Settings as SettingsIcon,
  Home as HomeIcon,
  Pdf as PdfIcon,
  HtmlReference as HtmlIcon
} from '@vicons/carbon'

const router = useRouter()
const activeKey = ref<string | null>('home')

// 渲染图标的辅助函数
function renderIcon(icon: any) {
  return () => h(NIcon, null, { default: () => h(icon) })
}

// 核心：菜单配置
const menuOptions: MenuOption[] = [
  {
    label: '主页',
    key: 'home',
    icon: renderIcon(HomeIcon)
  },
  {
    label: '文本工具',
    key: 'text',
    icon: renderIcon(TextIcon),
    children: [
      { label: '哈希计算', key: 'text-hash' }, // 对应路由 name
      { label: 'JSON 格式化', key: 'text-json', disabled: true } // 还没做可以先 disable
    ]
  },
  {
    label: '文档转换', // 第一级菜单
    key: 'doc',
    icon: renderIcon(DocIcon),
    children: [
      {
        label: 'PDF 转 Word', // 第二级菜单
        key: 'pdf-word',
        icon: renderIcon(PdfIcon)
      },
      {
        label: 'HTML 转 PDF',
        key: 'html-pdf',
        icon: renderIcon(HtmlIcon)
      }
    ]
  },
  {
    type: 'divider', // 分割线，增加层次感
    key: 'd1'
  },
  {
    label: '系统设置',
    key: 'settings',
    icon: renderIcon(SettingsIcon)
  }
]

// 处理点击
const handleUpdateValue = (key: string) => {
  activeKey.value = key
  router.push({ name: key })
}
</script>

<template>
  <div class="flex flex-col h-full bg-[#0F172A]">
    <div class="h-16 flex items-center px-6 border-b border-slate-800/50">
      <div class="w-8 h-8 bg-cyan-500/20 rounded-lg flex items-center justify-center mr-3">
        <span class="text-cyan-400 font-bold">K</span>
      </div>
      <span class="text-lg font-bold tracking-wider text-slate-100">KRATE</span>
    </div>

    <div class="flex-1 py-4 overflow-y-auto custom-scrollbar">
      <n-menu
        :options="menuOptions"
        :value="activeKey"
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
/* 隐藏侧边栏滚动条但保留功能 */
.custom-scrollbar::-webkit-scrollbar {
  width: 0px;
}
</style>