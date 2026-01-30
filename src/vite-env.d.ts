/// <reference types="vite/client" />

// 扩展 Vue Router 的 RouteMeta 接口
import 'vue-router'

declare module 'vue-router' {
  interface RouteMeta {
    title?: string
    icon?: any
    hidden?: boolean
    desc?: string
    standalone?: boolean
  }
}

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}
