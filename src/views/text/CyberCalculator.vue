<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from 'vue'
import { NIcon, NScrollbar } from 'naive-ui'
import { Time, TrashCan, Rotate } from '@vicons/carbon'

// === 状态定义 ===
const expression = ref('') // 当前输入的表达式 (例如: 12 + 5 * 3)
const result = ref('0')    // 当前显示的结果/输入缓冲
const history = ref<{ expr: string; res: string }[]>([])
const isResultState = ref(false) // 标记当前是否刚计算完结果
const errorState = ref(false)

// 历史记录滚动条引用
const scrollbarRef = ref<any>(null)

// === 核心计算逻辑 ===
// 安全的执行数学表达式
const safeEvaluate = (expr: string) => {
  try {
    // 1. 预处理：替换符号为 JS 可识别的 Math 函数
    let evalStr = expr
      .replace(/×/g, '*')
      .replace(/÷/g, '/')
      .replace(/π/g, 'Math.PI')
      .replace(/e/g, 'Math.E')
      .replace(/sin/g, 'Math.sin')
      .replace(/cos/g, 'Math.cos')
      .replace(/tan/g, 'Math.tan')
      .replace(/log/g, 'Math.log10')
      .replace(/ln/g, 'Math.log')
      .replace(/√/g, 'Math.sqrt')
      .replace(/\^/g, '**')

    // 2. 使用 Function 构造器代替 eval，相对安全
    const res = new Function(`return ${evalStr}`)()

    // 3. 处理精度问题
    return parseFloat(res.toFixed(10)).toString()
  } catch (e) {
    throw new Error('Error')
  }
}

// === 操作处理 ===
const handleInput = (key: string) => {
  errorState.value = false

  // 1. 清除 (C)
  if (key === 'C') {
    expression.value = ''
    result.value = '0'
    isResultState.value = false
    return
  }

  // 2. 删除 (DEL)
  if (key === 'DEL') {
    if (isResultState.value) {
      expression.value = ''
      result.value = '0'
      isResultState.value = false
    } else {
      if (result.value.length > 1) {
        result.value = result.value.slice(0, -1)
      } else {
        result.value = '0'
      }
    }
    return
  }

  // 3. 等号 (=)
  if (key === '=') {
    if (!expression.value && result.value === '0') return

    // 构建完整表达式
    const fullExpr = expression.value + result.value
    try {
      const res = safeEvaluate(fullExpr)

      // 记录历史
      history.value.push({ expr: fullExpr, res })
      if (history.value.length > 50) history.value.shift()

      // 更新状态
      expression.value = ''
      result.value = res
      isResultState.value = true

      // 滚动到底部
      nextTick(() => scrollbarRef.value?.scrollTo({ top: 99999, behavior: 'smooth' }))

    } catch (e) {
      result.value = 'Err'
      errorState.value = true
      isResultState.value = true
    }
    return
  }

  // 4. 运算符 (+ - * / ...)
  if (['+', '-', '×', '÷', '^', '%'].includes(key)) {
    if (isResultState.value && !errorState.value) {
      // 如果刚算完，把结果作为下一个表达式的开头
      expression.value = result.value + ' ' + key + ' '
      result.value = '0'
      isResultState.value = false
    } else {
      // 正常连接
      expression.value += result.value + ' ' + key + ' '
      result.value = '0'
    }
    return
  }

  // 5. 科学函数 (sin, cos...) 直接包裹当前数字
  if (['sin', 'cos', 'tan', 'log', 'ln', '√'].includes(key)) {
    result.value = `${key}(${result.value})`
    return
  }

  // 6. 数字输入
  if (isResultState.value) {
    // 如果刚算完，输入数字则开始新计算
    result.value = key
    expression.value = ''
    isResultState.value = false
  } else {
    if (result.value === '0' && key !== '.') {
      result.value = key
    } else {
      // 防止多个小数点
      if (key === '.' && result.value.includes('.')) return
      result.value += key
    }
  }
}

// === 键盘监听 ===
const handleKeydown = (e: KeyboardEvent) => {
  const keyMap: Record<string, string> = {
    'Enter': '=',
    '=': '=',
    'Escape': 'C',
    'Backspace': 'DEL',
    'Delete': 'DEL',
    '/': '÷',
    '*': '×',
    'x': '×',
  }

  // 数字和基本符号
  if (/^[0-9.+-]$/.test(e.key)) {
    handleInput(e.key)
    return
  }

  if (keyMap[e.key]) {
    handleInput(keyMap[e.key])
  }
}

onMounted(() => window.addEventListener('keydown', handleKeydown))
onUnmounted(() => window.removeEventListener('keydown', handleKeydown))

// 历史记录点击回填
const restoreHistory = (res: string) => {
  if (isResultState.value) {
    expression.value = ''
  }
  result.value = res
  isResultState.value = false
}

// 按钮布局配置
const buttons = [
  { label: 'C', class: 'text-red-400 font-bold border-red-500/30' },
  { label: 'DEL', class: 'text-orange-400' },
  { label: '%', class: 'text-cyan-400' },
  { label: '÷', class: 'text-cyan-400 font-bold text-xl' },

  { label: 'sin', class: 'text-xs text-slate-400' },
  { label: 'cos', class: 'text-xs text-slate-400' },
  { label: 'tan', class: 'text-xs text-slate-400' },
  { label: '×', class: 'text-cyan-400 font-bold text-xl' },

  { label: '7', class: 'text-slate-200 font-mono text-lg' },
  { label: '8', class: 'text-slate-200 font-mono text-lg' },
  { label: '9', class: 'text-slate-200 font-mono text-lg' },
  { label: '-', class: 'text-cyan-400 font-bold text-xl' },

  { label: '4', class: 'text-slate-200 font-mono text-lg' },
  { label: '5', class: 'text-slate-200 font-mono text-lg' },
  { label: '6', class: 'text-slate-200 font-mono text-lg' },
  { label: '+', class: 'text-cyan-400 font-bold text-xl' },

  { label: '1', class: 'text-slate-200 font-mono text-lg' },
  { label: '2', class: 'text-slate-200 font-mono text-lg' },
  { label: '3', class: 'text-slate-200 font-mono text-lg' },
  { label: '√', class: 'text-cyan-400' }, // 占位，或者把 = 搞大点

  { label: '0', class: 'col-span-2 text-slate-200 font-mono text-lg' },
  { label: '.', class: 'text-slate-200 font-mono text-lg' },
  { label: '=', class: 'bg-emerald-600/20 text-emerald-400 border-emerald-500/50 shadow-[0_0_15px_rgba(16,185,129,0.3)] hover:bg-emerald-600/40' },
]

const clearHistory = () => history.value = []
</script>

<template>
  <div class="h-full flex flex-col md:flex-row p-6 gap-6 bg-[#0F172A] overflow-hidden">

    <div class="flex-1 flex flex-col max-w-lg mx-auto w-full h-full">
      <div class="mb-6 p-6 rounded-2xl bg-slate-900 border border-slate-700/50 shadow-inner relative overflow-hidden group">
        <div class="absolute inset-0 bg-linear-to-br from-emerald-500/5 to-transparent pointer-events-none"></div>

        <div class="h-8 text-right text-slate-400 font-mono text-sm tracking-wider opacity-70 truncate">
          {{ expression }}
        </div>

        <div
          class="h-16 text-right font-mono text-5xl font-bold tracking-tight truncate transition-colors duration-200"
          :class="errorState ? 'text-red-400' : 'text-emerald-400 shadow-emerald-500/50'"
          style="text-shadow: 0 0 10px rgba(52, 211, 153, 0.3);"
        >
          {{ result }}
        </div>
      </div>

      <div class="flex-1 grid grid-cols-4 gap-4">
        <button
          v-for="btn in buttons"
          :key="btn.label"
          @click="handleInput(btn.label)"
          class="relative rounded-xl border border-slate-700/50 bg-slate-800/40
                 hover:bg-slate-700/60 hover:-translate-y-0.5 active:translate-y-0.5 active:scale-95
                 transition-all duration-150 flex items-center justify-center select-none"
          :class="[btn.class]"
        >
          {{ btn.label }}
        </button>
      </div>
    </div>

    <div class="w-80 hidden md:flex flex-col bg-slate-900/50 rounded-2xl border border-slate-800/50 overflow-hidden backdrop-blur-sm">
      <div class="p-4 border-b border-slate-800/50 flex justify-between items-center">
        <div class="flex items-center gap-2 text-slate-300 font-bold">
          <NIcon><Time /></NIcon>
          <span>Tape Log</span>
        </div>
        <button @click="clearHistory" class="p-1 hover:text-red-400 text-slate-500 transition-colors" title="清除历史">
          <NIcon><TrashCan /></NIcon>
        </button>
      </div>

      <NScrollbar ref="scrollbarRef" class="flex-1 p-4">
        <div v-if="history.length === 0" class="h-full flex flex-col items-center justify-center text-slate-600 space-y-2">
          <NIcon size="32" class="opacity-20"><Rotate /></NIcon>
          <span class="text-xs">暂无计算记录</span>
        </div>
        <div
          v-else
          v-for="(item, idx) in history"
          :key="idx"
          class="mb-4 p-3 rounded-lg hover:bg-slate-800/50 cursor-pointer group transition-colors border border-transparent hover:border-slate-700"
          @click="restoreHistory(item.res)"
        >
          <div class="text-right text-xs text-slate-500 font-mono mb-1 break-all group-hover:text-slate-400">
            {{ item.expr }}
          </div>
          <div class="text-right text-lg text-emerald-500/80 font-mono font-bold group-hover:text-emerald-400">
            = {{ item.res }}
          </div>
        </div>
      </NScrollbar>
    </div>

  </div>
</template>

<style scoped>
/* 霓虹按钮按下的发光效果 */
button:active::after {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: 0.75rem;
  box-shadow: 0 0 15px currentColor;
  opacity: 0.2;
}
</style>