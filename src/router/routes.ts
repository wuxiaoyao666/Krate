import { h, Component } from 'vue'
import { NIcon } from 'naive-ui'

import {
  Code as TextIcon,
  Document as DocIcon,
  Settings as SettingsIcon,
  Home as HomeIcon,
  Pdf as PdfIcon,
  HtmlReference as HtmlIcon,
} from '@vicons/carbon'
import { RouterView } from 'vue-router'

// 辅助函数：渲染图标
function renderIcon(icon: Component) {
  return () => h(NIcon, null, { default: () => h(icon) })
}

// 2. 定义路由配置
export const routes = [
  {
    path: '/',
    name: 'home',
    component: () => import('@/views/text/HashTool.vue'),
    meta: {
      title: '主页',
      icon: renderIcon(HomeIcon),
      // 可以在这里加 hidden: true 如果不想在菜单显示
    },
  },
  {
    path: '/text',
    name: 'text',
    component: { render: () => h(RouterView) },
    meta: {
      title: '文本工具',
      icon: renderIcon(TextIcon),
    },
    children: [
      {
        path: 'hash',
        name: 'text-hash',
        component: () => import('@/views/text/HashTool.vue'),
        meta: { title: '哈希计算' },
      },
      {
        path: 'json',
        name: 'text-json',
        component: () => import('@/views/text/JsonTool.vue'),
        meta: { title: 'JSON 格式化' },
      },
    ],
  },
  {
    path: '/doc',
    name: 'doc',
    component: { render: () => h(RouterView) },
    meta: {
      title: '文档转换',
      icon: renderIcon(DocIcon),
    },
    children: [
      {
        path: 'pdf-word',
        name: 'pdf-word',
        component: () => import('@/views/text/HashTool.vue'), // 暂时占位
        meta: { title: 'PDF 转 Word', icon: renderIcon(PdfIcon) },
      },
      {
        path: 'html-pdf',
        name: 'html-pdf',
        component: () => import('@/views/text/HashTool.vue'), // 暂时占位
        meta: { title: 'HTML 转 PDF', icon: renderIcon(HtmlIcon) },
      },
    ],
  },
  {
    path: '/settings',
    name: 'settings',
    component: () => import('@/views/text/HashTool.vue'), // 暂时占位
    meta: {
      title: '系统设置',
      icon: renderIcon(SettingsIcon),
    },
  },
]
