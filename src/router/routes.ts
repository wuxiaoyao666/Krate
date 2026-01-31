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
  TableSplit,
  Network4,
  Plug,
  Package,
  Screen,
  Copy,
} from '@vicons/carbon'
import { RouteRecordRaw, RouterView } from 'vue-router'

// 辅助函数：渲染图标
function renderIcon(icon: Component) {
  return () => h(NIcon, null, { default: () => h(icon) })
}

// 2. 定义路由配置
export const routes: RouteRecordRaw[] = [
  {
    path: '/',
    name: 'home',
    component: () => import('@/views/Home.vue'),
    meta: {
      title: '主页',
      icon: renderIcon(HomeIcon),
    },
  },
  {
    path: '/system',
    name: 'system',
    component: { render: () => h(RouterView) },
    meta: {
      title: '系统工具',
      icon: renderIcon(Network4),
    },
    children: [
      {
        path: 'info',
        name: 'system-info',
        component: () => import('@/views/system/SystemInfo.vue'),
        meta: {
          title: '系统监控',
          icon: renderIcon(Screen),
          desc: '实时查看 CPU、内存、系统负载',
        },
      },
      {
        path: 'port',
        name: 'system-port',
        component: () => import('@/views/system/PortManager.vue'),
        meta: {
          title: '端口探针',
          icon: renderIcon(Plug),
          desc: '扫描本地端口占用情况并一键杀进程',
        },
      },
      {
        path: 'archive',
        name: 'system-archive',
        component: () => import('@/views/system/KrateArchive.vue'),
        meta: {
          title: '私有归档',
          icon: renderIcon(Package),
          desc: '打包生成专属 .krate 格式文件',
        },
      },
      {
        path: 'clipboard',
        name: 'system-clipboard',
        component: () => import('@/views/system/ClipboardHistory.vue'),
        meta: {
          title: '剪切板历史',
          icon: renderIcon(Copy),
          desc: '自动记录复制历史，支持一键回填',
        },
      },
    ],
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
        path: 'codec',
        name: 'text-codec',
        component: () => import('@/views/text/CodecTool.vue'),
        meta: {
          title: '编码解码',
          icon: renderIcon(Code),
          desc: 'Base64 / URL / Unicode 编解码转换',
        },
      },
      {
        path: 'hash',
        name: 'text-hash',
        component: () => import('@/views/text/HashTool.vue'),
        meta: { title: '哈希计算', icon: renderIcon(Locked), desc: '常用摘要计算，支持 Bcrypt' },
      },
      {
        path: 'json',
        name: 'text-json',
        component: () => import('@/views/text/JsonTool.vue'),
        meta: {
          title: 'JSON 格式化',
          icon: renderIcon(Terminal),
          desc: 'JSON 美化、压缩、去转义与语法校验',
        },
      },
      {
        path: 'diff',
        name: 'text-diff',
        component: () => import('@/views/text/DiffTool.vue'),
        meta: { title: '文本对比', icon: renderIcon(Compare), desc: '代码与文本Diff' },
      },
      {
        path: 'password',
        name: 'text-password',
        component: () => import('@/views/text/PasswordGen.vue'),
        meta: {
          title: '密码生成',
          icon: renderIcon(Password),
          desc: '生成高强度、自定义规则的随机密码',
        },
      },
      {
        path: 'jwt',
        name: 'text-jwt',
        component: () => import('@/views/text/JwtParser.vue'),
        meta: { title: 'JWT 解析器', icon: renderIcon(Code) },
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
      desc: '图像处理与生成工具',
    },
    children: [
      {
        path: 'compress',
        name: 'image-compress',
        component: () => import('@/views/image/CompressTool.vue'),
        meta: {
          title: '图片压缩',
          icon: renderIcon(FitToScreen),
          desc: 'PNG / JPG / WebP 图片无损或有损压缩',
        },
      },
      {
        path: 'crop',
        name: 'image-crop',
        component: () => import('@/views/image/ImageCropper.vue'),
        meta: { title: '图片裁剪', icon: renderIcon(Crop), desc: '图片自由裁剪、旋转与比例调整' },
      },
      {
        path: 'resize',
        name: 'image-resize',
        component: () => import('@/views/image/ImageResizer.vue'),
        meta: {
          title: '尺寸调整',
          icon: renderIcon(Scale),
          desc: '批量调整图片分辨率与文件尺寸',
        },
      },
      {
        path: 'qrcode',
        name: 'image-qrcode',
        component: () => import('@/views/image/QrGenerator.vue'),
        meta: { title: '二维码生成', icon: renderIcon(QrCode) },
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
      desc: '文档与办公辅助工具',
    },
    children: [
      {
        path: 'pdf-protect',
        name: 'doc-protect',
        component: () => import('@/views/doc/PdfProtect.vue'),
        meta: {
          title: 'PDF 加解密',
          icon: renderIcon(PdfIcon),
          desc: 'PDF 文档添加密码保护或移除密码',
        },
      },
    ],
  },
  {
    path: '/games',
    name: 'games',
    component: () => import('@/views/games/GameHall.vue'),
    meta: {
      title: '游戏',
      icon: renderIcon(GameConsole),
    },
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
  {
    path: '/game/minesweeper',
    name: 'game-minesweeper',
    component: () => import('@/views/games/Minesweeper.vue'),
    meta: { title: '扫雷', hidden: true, standalone: true },
  },
  {
    path: '/game/2048',
    name: 'game-2048',
    component: () => import('@/views/games/Game2048.vue'),
    meta: {
      title: '2048',
      icon: renderIcon(TableSplit),
      hidden: true,
      standalone: true, // 独立窗口模式
    },
  },
]
