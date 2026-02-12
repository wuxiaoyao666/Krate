<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import {
  NAlert,
  NButton,
  NCard,
  NDivider,
  NFormItem,
  NIcon,
  NInput,
  NInputNumber,
  NSwitch,
  NTag,
  useMessage
} from 'naive-ui'
import { Add, Play, Renew, Stop, TrashCan } from '@vicons/carbon'

interface ProxyRouteForm {
  id: string
  name: string
  enabled: boolean
  host: string
  pathPrefix: string
  target: string
  stripPrefix: boolean
  allowInsecureTls: boolean
}

interface ProxyStatus {
  running: boolean
  listenHost: string | null
  listenPort: number | null
  routeCount: number
  totalRequests: number
  startedAt: number | null
  lastError: string | null
  message: string
}

interface ProxyStartRequest {
  listenHost: string
  listenPort: number
  routes: ProxyRouteForm[]
}

interface ProxyStoredConfig {
  listenHost: string
  listenPort: number | null
  routes: ProxyRouteForm[]
}

const message = useMessage()
const STORAGE_KEY = 'krate.proxy.config.v1'

const loadingStatus = ref(false)
const submitting = ref(false)
const pollTimer = ref<number | null>(null)

const listenHost = ref('0.0.0.0')
const listenPort = ref<number | null>(8080)
const routes = ref<ProxyRouteForm[]>([
  {
    id: `route-${Date.now()}`,
    name: '本地服务',
    enabled: true,
    host: '*',
    pathPrefix: '/',
    target: 'http://127.0.0.1:3000',
    stripPrefix: false,
    allowInsecureTls: false
  }
])
const status = ref<ProxyStatus | null>(null)

const enabledRouteCount = computed(() => routes.value.filter(item => item.enabled).length)

const startedAtText = computed(() => {
  const timestamp = status.value?.startedAt
  if (!timestamp) return '--'
  return new Date(timestamp * 1000).toLocaleString()
})

const createRoute = (): ProxyRouteForm => ({
  id: `route-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
  name: '',
  enabled: true,
  host: '',
  pathPrefix: '/',
  target: 'http://127.0.0.1:3000',
  stripPrefix: false,
  allowInsecureTls: false
})

const normalizeRoute = (route: Partial<ProxyRouteForm>, index: number): ProxyRouteForm => ({
  id: route.id || `route-${Date.now()}-${index}`,
  name: (route.name || '').toString(),
  enabled: route.enabled !== false,
  host: (route.host || '').toString(),
  pathPrefix: normalizePathPrefix((route.pathPrefix || '/').toString()),
  target: (route.target || 'http://127.0.0.1:3000').toString(),
  stripPrefix: Boolean(route.stripPrefix),
  allowInsecureTls: Boolean(route.allowInsecureTls)
})

const normalizePathPrefix = (pathPrefix: string) => {
  const raw = pathPrefix.trim()
  if (!raw) return '/'
  const withSlash = raw.startsWith('/') ? raw : `/${raw}`
  return withSlash.length > 1 ? withSlash.replace(/\/+$/, '') : withSlash
}

const sanitizeRoutes = (list: ProxyRouteForm[]) => {
  return list.map(route => ({
    ...route,
    name: route.name.trim(),
    host: route.host.trim(),
    pathPrefix: normalizePathPrefix(route.pathPrefix),
    target: route.target.trim(),
    allowInsecureTls: Boolean(route.allowInsecureTls)
  }))
}

const persistConfig = () => {
  const payload: ProxyStoredConfig = {
    listenHost: listenHost.value.trim() || '127.0.0.1',
    listenPort: listenPort.value,
    routes: routes.value.map((route, index) => normalizeRoute(route, index))
  }
  localStorage.setItem(STORAGE_KEY, JSON.stringify(payload))
}

const restoreConfig = () => {
  const raw = localStorage.getItem(STORAGE_KEY)
  if (!raw) return false

  try {
    const parsed = JSON.parse(raw) as Partial<ProxyStoredConfig>
    if (typeof parsed.listenHost === 'string' && parsed.listenHost.trim()) {
      listenHost.value = parsed.listenHost.trim()
    }
    if (
      parsed.listenPort === null ||
      (typeof parsed.listenPort === 'number' &&
        Number.isInteger(parsed.listenPort) &&
        parsed.listenPort > 0 &&
        parsed.listenPort <= 65535)
    ) {
      listenPort.value = parsed.listenPort
    }
    if (Array.isArray(parsed.routes) && parsed.routes.length > 0) {
      routes.value = parsed.routes.map((route, index) => normalizeRoute(route, index))
    }
    return true
  } catch (error) {
    console.warn('Failed to restore proxy config', error)
    return false
  }
}

const refreshStatus = async (options: { silent?: boolean; syncForm?: boolean; showLoading?: boolean } = {}) => {
  const { silent = false, syncForm = false, showLoading = true } = options
  if (showLoading) {
    loadingStatus.value = true
  }
  try {
    status.value = await invoke<ProxyStatus>('proxy_get_status')

    if (syncForm && status.value.listenHost) {
      listenHost.value = status.value.listenHost
    }
    if (syncForm && status.value.listenPort) {
      listenPort.value = status.value.listenPort
    }
  } catch (error) {
    if (!silent) {
      message.error(`获取代理状态失败: ${error}`)
    }
  } finally {
    if (showLoading) {
      loadingStatus.value = false
    }
  }
}

const addRoute = () => {
  routes.value.push(createRoute())
}

const removeRoute = (id: string) => {
  routes.value = routes.value.filter(route => route.id !== id)
}

const applyProxy = async () => {
  const port = Number(listenPort.value)
  if (!listenHost.value.trim()) {
    message.error('监听地址不能为空')
    return
  }
  if (!Number.isInteger(port) || port <= 0 || port > 65535) {
    message.error('监听端口必须在 1 - 65535 之间')
    return
  }

  const nextRoutes = sanitizeRoutes(routes.value)
  if (nextRoutes.some(route => route.enabled && !route.target)) {
    message.error('启用中的路由必须填写上游目标地址')
    return
  }
  if (!nextRoutes.some(route => route.enabled)) {
    message.error('至少启用一条路由')
    return
  }

  routes.value = nextRoutes
  persistConfig()
  submitting.value = true
  try {
    if (status.value?.running) {
      await invoke('proxy_stop')
    }

    const payload: ProxyStartRequest = {
      listenHost: listenHost.value.trim(),
      listenPort: port,
      routes: routes.value
    }

    status.value = await invoke<ProxyStatus>('proxy_start', { config: payload })
    message.success('反向代理已启动')
    persistConfig()
  } catch (error) {
    message.error(`启动失败: ${error}`)
    await refreshStatus()
  } finally {
    submitting.value = false
  }
}

const stopProxy = async () => {
  submitting.value = true
  try {
    status.value = await invoke<ProxyStatus>('proxy_stop')
    message.success('反向代理已停止')
    persistConfig()
  } catch (error) {
    message.error(`停止失败: ${error}`)
  } finally {
    submitting.value = false
  }
}

const startPolling = () => {
  if (pollTimer.value !== null) return
  pollTimer.value = window.setInterval(() => {
    void refreshStatus({ silent: true, showLoading: false })
  }, 1200)
}

const stopPolling = () => {
  if (pollTimer.value === null) return
  window.clearInterval(pollTimer.value)
  pollTimer.value = null
}

watch([listenHost, listenPort, routes], () => {
  persistConfig()
}, { deep: true })

onMounted(() => {
  const restored = restoreConfig()
  void refreshStatus({ syncForm: !restored })
  startPolling()
})

onUnmounted(() => {
  stopPolling()
})
</script>

<template>
  <div class="h-full flex flex-col p-6 space-y-5 max-w-6xl mx-auto">
    <div class="flex items-center justify-between">
      <div>
        <h2 class="text-2xl font-bold text-slate-100">反向代理服务</h2>
        <p class="text-slate-400 text-sm mt-1">本地可视化管理 HTTP / HTTPS 代理路由，支持一键启停</p>
      </div>
      <div class="flex items-center gap-3">
        <n-button secondary :loading="loadingStatus" @click="refreshStatus({ syncForm: true })">
          <template #icon>
            <n-icon :component="Renew" />
          </template>
          刷新状态
        </n-button>
        <n-button type="primary" :loading="submitting" @click="applyProxy">
          <template #icon>
            <n-icon :component="Play" />
          </template>
          {{ status?.running ? '重启并应用' : '启动代理' }}
        </n-button>
        <n-button type="error" secondary :disabled="!status?.running" :loading="submitting" @click="stopProxy">
          <template #icon>
            <n-icon :component="Stop" />
          </template>
          停止
        </n-button>
      </div>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-3 gap-5">
      <n-card class="lg:col-span-2 bg-slate-900/60 border border-slate-700/80">
        <template #header>监听配置</template>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <n-form-item label="监听地址">
            <n-input v-model:value="listenHost" placeholder="127.0.0.1 / 0.0.0.0" />
          </n-form-item>
          <n-form-item label="监听端口">
            <n-input-number v-model:value="listenPort" :min="1" :max="65535" class="w-full" />
          </n-form-item>
        </div>

        <n-divider />

        <div class="flex flex-wrap gap-3">
          <n-tag :type="status?.running ? 'success' : 'warning'" size="small" bordered>
            {{ status?.running ? '运行中' : '未启动' }}
          </n-tag>
          <n-tag size="small" bordered>启用路由: {{ enabledRouteCount }}</n-tag>
          <n-tag size="small" bordered>转发请求: {{ status?.totalRequests || 0 }}</n-tag>
          <n-tag size="small" bordered>启动时间: {{ startedAtText }}</n-tag>
        </div>

        <p class="text-slate-400 text-xs mt-3">
          当前状态：{{ status?.message || '等待状态同步...' }}
        </p>
      </n-card>

      <n-card class="bg-slate-900/60 border border-slate-700/80">
        <template #header>使用说明</template>
        <n-alert type="info" :show-icon="false">
          <div class="text-xs leading-6">
            1. 支持 HTTP / HTTPS / WS / WSS 上游地址。<br />
            2. `Host` 留空表示匹配所有域名。<br />
            3. `路径前缀` 支持按最长前缀匹配。<br />
            4. `剥离前缀` 开启后，转发时会移除该前缀。<br />
            5. 支持 WebSocket 透传（`ws://` 与 `wss://` 上游）。
          </div>
        </n-alert>
        <p v-if="status?.lastError" class="text-rose-400 text-xs mt-3">
          最近错误：{{ status.lastError }}
        </p>
      </n-card>
    </div>

    <n-card class="bg-slate-900/60 border border-slate-700/80">
      <template #header>
        <div class="flex items-center justify-between">
          <span>路由规则</span>
          <n-button size="small" secondary @click="addRoute">
            <template #icon>
              <n-icon :component="Add" />
            </template>
            添加规则
          </n-button>
        </div>
      </template>

      <div v-if="routes.length === 0" class="py-8 text-center text-slate-500 text-sm">
        暂无规则，请先添加一条代理路由。
      </div>

      <div v-else class="space-y-4">
        <div
          v-for="(route, index) in routes"
          :key="route.id"
          class="rounded-xl border border-slate-700/80 p-4 bg-slate-950/40"
        >
          <div class="flex items-center justify-between mb-3">
            <div class="text-sm font-medium text-slate-200">
              规则 #{{ index + 1 }} {{ route.name ? `· ${route.name}` : '' }}
            </div>
            <div class="flex items-center gap-3">
              <span class="text-xs text-slate-400">启用</span>
              <n-switch v-model:value="route.enabled" />
              <n-button size="tiny" quaternary type="error" @click="removeRoute(route.id)">
                <template #icon>
                  <n-icon :component="TrashCan" />
                </template>
              </n-button>
            </div>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-3">
            <n-form-item label="规则名称" class="mb-0">
              <n-input v-model:value="route.name" placeholder="可选，例如 API 服务" />
            </n-form-item>
            <n-form-item label="Host 匹配" class="mb-0">
              <n-input v-model:value="route.host" placeholder="留空或 * 表示全部" />
            </n-form-item>
            <n-form-item label="路径前缀" class="mb-0">
              <n-input v-model:value="route.pathPrefix" placeholder="/api" />
            </n-form-item>
            <n-form-item label="上游目标" class="mb-0">
              <n-input v-model:value="route.target" placeholder="http://127.0.0.1:3000 / https://api.example.com / wss://socket.example.com" />
            </n-form-item>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-3 mt-3">
            <div class="flex items-center gap-3">
              <span class="text-xs text-slate-400">剥离路径前缀</span>
              <n-switch v-model:value="route.stripPrefix" />
              <span class="text-xs text-slate-500">
                `/api/user` 转发为 `/user`
              </span>
            </div>
            <div class="flex items-center gap-3">
              <span class="text-xs text-slate-400">忽略 HTTPS 证书校验</span>
              <n-switch v-model:value="route.allowInsecureTls" />
              <span class="text-xs text-slate-500">
                仅对 HTTPS/WSS 上游生效（仅建议开发调试时开启）
              </span>
            </div>
          </div>
        </div>
      </div>
    </n-card>
  </div>
</template>
