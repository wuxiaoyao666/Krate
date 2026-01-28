import { createRouter, createWebHistory } from 'vue-router'
import { h } from 'vue'

// 临时占位组件，方便演示
const Placeholder = (title: string) => ({
  render: () => h('div', { class: 'text-slate-400' }, [
    h('h1', { class: 'text-3xl font-bold text-white mb-4' }, title),
    h('p', '该功能正在开发中...')
  ])
})

const routes = [
  { path: '/', name: 'home', component: Placeholder('欢迎使用 Krate') },
  { path: '/text/hash', name: 'text-hash', component: Placeholder('文本哈希工具') },
  { path: '/doc/pdf-word', name: 'pdf-word', component: Placeholder('PDF 转 Word') },
  { path: '/doc/html-pdf', name: 'html-pdf', component: Placeholder('HTML 转 PDF') },
  { path: '/settings', name: 'settings', component: Placeholder('系统设置') },
]

const router = createRouter({
  history: createWebHistory(),
  routes
})

export default router