/// <reference types="vite/client" />

// 扩展 Vue Router 的 RouteMeta 接口
import 'vue-router'
import type { DefineComponent, VNodeChild } from 'vue'

declare module 'vue-router' {
  interface RouteMeta {
    title?: string
    icon?: () => VNodeChild
    hidden?: boolean
    desc?: string
    standalone?: boolean
  }
}

declare module '*.vue' {
  const component: DefineComponent<Record<string, never>, Record<string, never>, unknown>
  export default component
}
