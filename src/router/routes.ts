import { h, Component } from 'vue'
import { NIcon } from 'naive-ui'

import {
  Code as TextIcon,
  Document as DocIcon,
  Settings as SettingsIcon,
  Home as HomeIcon,
  Pdf as PdfIcon,
  Compare,
  Locked,
  Terminal,
  Password,
  ImageReference,
  FitToScreen,
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
        meta: { title: '哈希计算', icon: renderIcon(Locked) },
      },
      {
        path: 'json',
        name: 'text-json',
        component: () => import('@/views/text/JsonTool.vue'),
        meta: { title: 'JSON 格式化', icon: renderIcon(Terminal) },
      },
      {
        path: 'diff',
        name: 'text-diff',
        component: () => import('@/views/text/DiffTool.vue'),
        meta: { title: '文本对比', icon: renderIcon(Compare) },
      },
      {
        path: 'password',
        name: 'text-password',
        component: () => import('@/views/text/PasswordGen.vue'),
        meta: { title: '密码生成', icon: renderIcon(Password) },
      },
    ],
  },
  {
    path: '/image',
    name: 'image',
    component: { render: () => h(RouterView) },
    meta: {
      title: '图片工具',
      icon: renderIcon(ImageReference),
    },
    children: [
      {
        path: 'compress',
        name: 'image-compress',
        component: () => import('@/views/image/CompressTool.vue'),
        meta: { title: '图片压缩', icon: renderIcon(FitToScreen) },
      },
    ],
  },
  {
    path: '/doc',
    name: 'doc',
    component: { render: () => h(RouterView) },
    meta: {
      title: '文档处理',
      icon: renderIcon(DocIcon),
    },
    children: [
      {
        path: 'pdf-protect',
        name: 'doc-protect',
        component: () => import('@/views/doc/PdfProtect.vue'),
        meta: { title: 'PDF 加解密', icon: renderIcon(PdfIcon) },
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
