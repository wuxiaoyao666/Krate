<script setup lang="ts">
import {
  NConfigProvider,
  NGlobalStyle,
  NLayout,
  NLayoutSider,
  NLayoutContent,
  darkTheme,
  NMessageProvider,
  type GlobalThemeOverrides,
} from 'naive-ui'
import Sidebar from './components/Sidebar.vue'
import { useRoute } from 'vue-router'
import { useClipboard } from '@/hooks/useClipboard'
import { onMounted } from 'vue'

// 获取当前路由
const route = useRoute()

const themeOverrides: GlobalThemeOverrides = {
  common: {
    primaryColor: '#22D3EE',
    primaryColorHover: '#67E8F9',
    primaryColorPressed: '#06B6D4',
  },
  Menu: {
    itemColorActive: 'rgba(34, 211, 238, 0.1)', // 选中项背景极淡的青色
    itemTextColorActive: '#22D3EE', // 选中项文字颜色
    itemIconColorActive: '#22D3EE',
    itemTextColor: '#94A3B8', // 默认文字 Slate-400
    itemIconColor: '#94A3B8',
    itemTextColorHover: '#F1F5F9', // 悬停文字变白
    itemIconColorHover: '#F1F5F9',
    borderRadius: '8px', // 菜单项圆角
  },
  Layout: {
    siderColor: '#0F172A', // 侧边栏背景
    color: '#1E293B', // 内容区背景
  },
}

// 启动全局监听
const { initClipboard } = useClipboard()

onMounted(() => {
  initClipboard()
})

if (import.meta.env.PROD) {
  // 禁止右键菜单
  document.addEventListener('contextmenu', (event) => {
    event.preventDefault()
  })
}
</script>

<template>
  <n-config-provider :theme="darkTheme" :theme-overrides="themeOverrides">
    <n-global-style />
    <n-message-provider>
      <!--   独立窗口运行   -->
      <div v-if="route.meta.standalone" class="h-screen w-screen bg-[#0F172A] overflow-hidden">
        <router-view />
      </div>

      <n-layout v-else has-sider class="h-screen w-screen bg-[#0F172A]">
        <n-layout-sider
          bordered
          collapse-mode="width"
          :collapsed-width="64"
          :width="240"
          class="border-r border-slate-800"
        >
          <Sidebar />
        </n-layout-sider>

        <n-layout-content content-style="padding: 0;">
          <div class="h-full p-2 bg-[#0F172A]">
            <div
              class="h-full w-full bg-[#1E293B] rounded-2xl shadow-2xl overflow-hidden flex flex-col relative border border-slate-700/30"
            >
              <div data-tauri-drag-region class="h-8 w-full absolute top-0 left-0 z-50"></div>

              <div class="flex-1 p-8 overflow-y-auto">
                <router-view v-slot="{ Component }">
                  <transition name="fade" mode="out-in">
                    <component :is="Component" />
                  </transition>
                </router-view>
              </div>
            </div>
          </div>
        </n-layout-content>
      </n-layout>
    </n-message-provider>
  </n-config-provider>
</template>

<style>
/* 页面切换动画 */
.fade-enter-active,
.fade-leave-active {
  transition:
    opacity 0.2s ease,
    transform 0.2s ease;
}

.fade-enter-from {
  opacity: 0;
  transform: translateY(10px);
}

.fade-leave-to {
  opacity: 0;
  transform: translateY(-10px);
}
</style>
