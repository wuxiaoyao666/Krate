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
  Scale,
  Code,
  QrCode,
  Crop,
  GameConsole,
  TableSplit, Network4, Plug
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
      icon: renderIcon(HomeIcon)
      // 可以在这里加 hidden: true 如果不想在菜单显示
    }
  },
  {
    path: '/system',
    name: 'system',
    component: { render: () => h(RouterView) },
    meta: {
      title: '系统工具',
      icon: renderIcon(Network4)
    },
    children: [
      {
        path: 'port',
        name: 'system-port',
        component: () => import('@/views/system/PortManager.vue'),
        meta: { title: '端口探针', icon: renderIcon(Plug) }
      }
    ]
  },
  {
    path: '/text',
    name: 'text',
    component: { render: () => h(RouterView) },
    meta: {
      title: '文本工具',
      icon: renderIcon(TextIcon)
    },
    children: [
      {
        path: 'codec',
        name: 'text-codec',
        component: () => import('@/views/text/CodecTool.vue'),
        meta: { title: '编码解码', icon: renderIcon(Code) }
      },
      {
        path: 'hash',
        name: 'text-hash',
        component: () => import('@/views/text/HashTool.vue'),
        meta: { title: '哈希计算', icon: renderIcon(Locked) }
      },
      {
        path: 'json',
        name: 'text-json',
        component: () => import('@/views/text/JsonTool.vue'),
        meta: { title: 'JSON 格式化', icon: renderIcon(Terminal) }
      },
      {
        path: 'diff',
        name: 'text-diff',
        component: () => import('@/views/text/DiffTool.vue'),
        meta: { title: '文本对比', icon: renderIcon(Compare) }
      },
      {
        path: 'password',
        name: 'text-password',
        component: () => import('@/views/text/PasswordGen.vue'),
        meta: { title: '密码生成', icon: renderIcon(Password) }
      },
      {
        path: 'jwt',
        name: 'text-jwt',
        component: () => import('@/views/text/JwtParser.vue'),
        meta: { title: 'JWT 解析器', icon: renderIcon(Code) }
      }
    ]
  },
  {
    path: '/image',
    name: 'image',
    component: { render: () => h(RouterView) },
    meta: {
      title: '图片工具',
      icon: renderIcon(ImageReference)
    },
    children: [
      {
        path: 'compress',
        name: 'image-compress',
        component: () => import('@/views/image/CompressTool.vue'),
        meta: { title: '图片压缩', icon: renderIcon(FitToScreen) }
      },
      {
        path: 'crop',
        name: 'image-crop',
        component: () => import('@/views/image/ImageCropper.vue'),
        meta: { title: '图片裁剪', icon: renderIcon(Crop) }
      },
      {
        path: 'resize',
        name: 'image-resize',
        component: () => import('@/views/image/ImageResizer.vue'),
        meta: {
          title: '尺寸调整',
          icon: renderIcon(Scale)
        }
      },
      {
        path: 'qrcode',
        name: 'image-qrcode',
        component: () => import('@/views/image/QrGenerator.vue'),
        meta: { title: '二维码生成', icon: renderIcon(QrCode) }
      }
    ]
  },
  {
    path: '/doc',
    name: 'doc',
    component: { render: () => h(RouterView) },
    meta: {
      title: '文档处理',
      icon: renderIcon(DocIcon)
    },
    children: [
      {
        path: 'pdf-protect',
        name: 'doc-protect',
        component: () => import('@/views/doc/PdfProtect.vue'),
        meta: { title: 'PDF 加解密', icon: renderIcon(PdfIcon) }
      }
    ]
  },
  {
    path: '/games',
    name: 'games',
    component: () => import('@/views/games/GameHall.vue'),
    meta: {
      title: '游戏',
      icon: renderIcon(GameConsole)
    }
  },
  {
    path: '/settings',
    name: 'settings',
    component: () => import('@/views/text/HashTool.vue'), // 暂时占位
    meta: {
      title: '系统设置',
      icon: renderIcon(SettingsIcon)
    }
  },
  {
    path: '/game/minesweeper',
    name: 'game-minesweeper',
    component: () => import('@/views/games/Minesweeper.vue'),
    meta: { title: '扫雷', hidden: true, standalone: true }
  },
  {
    path: '/game/2048',
    name: 'game-2048',
    component: () => import('@/views/games/Game2048.vue'),
    meta: {
      title: '2048',
      icon: renderIcon(TableSplit),
      hidden: true,
      standalone: true // 独立窗口模式
    }
  }
]
