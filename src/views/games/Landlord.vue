<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { NButton, NInput, NIcon, NAvatar, useMessage } from 'naive-ui'
import { 
  Wifi, 
  User, 
  Settings, 
  VolumeUpFilled, 
  VolumeMuteFilled,
  PlayFilled,
  ArrowLeft
} from '@vicons/carbon'

const message = useMessage()

// === Types ===
type GamePhase = 'lobby' | 'room' | 'playing' | 'gameover'
type CardSuit = 'spade' | 'heart' | 'club' | 'diamond' | 'joker'
type CardRank = '3'|'4'|'5'|'6'|'7'|'8'|'9'|'10'|'J'|'Q'|'K'|'A'|'2'|'S'|'B'

interface PokerCard {
  id: string
  suit: CardSuit
  rank: CardRank
  selected?: boolean
}

interface Player {
  id: string
  name: string
  avatar: string
  isHost: boolean
  isLandlord: boolean
  cardCount: number
  status: 'waiting' | 'ready' | 'playing'
}

// === State ===
const phase = ref<GamePhase>('lobby')
const soundEnabled = ref(true)

// Lobby State
const generateRandomName = () => {
  const adjs = ['快乐的', '神秘的', '无敌的', '传说的', '调皮的', '机智的', '暴躁的', '幸运的', '勇敢的']
  const nouns = ['地主', '农民', '老鼠', '老虎', '小猫', '小狗', '玩家', '高手', '菜鸟']
  return adjs[Math.floor(Math.random() * adjs.length)] + nouns[Math.floor(Math.random() * nouns.length)] + Math.floor(Math.random() * 100)
}
const myName = ref(generateRandomName())
const joinIp = ref('')

// Room State
const players = ref<Player[]>([
  { id: '1', name: myName.value, avatar: `https://api.dicebear.com/7.x/adventurer/svg?seed=${myName.value}`, isHost: true, isLandlord: false, cardCount: 0, status: 'ready' }
])

// Game State
const myCards = ref<PokerCard[]>([
  { id: 'c1', suit: 'spade', rank: 'A' },
  { id: 'c2', suit: 'heart', rank: 'K' },
  { id: 'c3', suit: 'club', rank: 'K' },
  { id: 'c4', suit: 'diamond', rank: 'Q' },
  { id: 'c5', suit: 'spade', rank: 'J' },
  { id: 'c6', suit: 'heart', rank: '10' },
  { id: 'c7', suit: 'club', rank: '9' },
  { id: 'c8', suit: 'diamond', rank: '8' },
  { id: 'c9', suit: 'spade', rank: '7' },
  { id: 'c10', suit: 'heart', rank: '6' },
  { id: 'c11', suit: 'club', rank: '5' },
  { id: 'c12', suit: 'diamond', rank: '4' },
  { id: 'c13', suit: 'spade', rank: '3' },
  { id: 'c14', suit: 'joker', rank: 'S' }, // Small Joker
  { id: 'c15', suit: 'joker', rank: 'B' }, // Big Joker
])

const tableCards = ref<PokerCard[]>([
  { id: 't1', suit: 'heart', rank: '4' },
  { id: 't2', suit: 'diamond', rank: '4' },
  { id: 't3', suit: 'club', rank: '4' },
  { id: 't4', suit: 'spade', rank: '3' },
])

const actionStatus = ref<'waiting' | 'call' | 'play'>('play')
const topCards = ref<PokerCard[]>([
  { id: 'top1', suit: 'spade', rank: '2' },
  { id: 'top2', suit: 'heart', rank: '2' },
  { id: 'top3', suit: 'club', rank: 'A' },
])

// === Actions ===
const createRoom = () => {
  players.value = [
    { id: '1', name: myName.value, avatar: `https://api.dicebear.com/7.x/adventurer/svg?seed=${myName.value}`, isHost: true, isLandlord: false, cardCount: 0, status: 'ready' }
  ]
  phase.value = 'room'
  message.success('房间创建成功，等待其他玩家加入...')
  
  // Mock players joining
  setTimeout(() => {
    players.value.push({ id: '2', name: 'Alice', avatar: `https://api.dicebear.com/7.x/adventurer/svg?seed=Alice`, isHost: false, isLandlord: false, cardCount: 0, status: 'ready' })
  }, 2000)
  setTimeout(() => {
    players.value.push({ id: '3', name: 'Bob', avatar: `https://api.dicebear.com/7.x/adventurer/svg?seed=Bob`, isHost: false, isLandlord: false, cardCount: 0, status: 'ready' })
  }, 4000)
}

const joinRoom = () => {
  if (!joinIp.value) {
    message.warning('请输入房主 IP 地址')
    return
  }
  message.loading('正在连接到房间...')
  setTimeout(() => {
    players.value = [
      { id: '2', name: 'Host_Alice', avatar: `https://api.dicebear.com/7.x/adventurer/svg?seed=Host_Alice`, isHost: true, isLandlord: false, cardCount: 0, status: 'ready' },
      { id: '1', name: myName.value, avatar: `https://api.dicebear.com/7.x/adventurer/svg?seed=${myName.value}`, isHost: false, isLandlord: false, cardCount: 0, status: 'ready' }
    ]
    phase.value = 'room'
    message.success('加入房间成功')
  }, 1000)
}

const startGame = () => {
  if (players.value.length < 3) {
    message.warning('需要 3 名玩家才能开始游戏')
    return
  }
  phase.value = 'playing'
  players.value.forEach(p => {
    p.cardCount = 17
    p.status = 'playing'
  })
}

const toggleCard = (card: PokerCard) => {
  card.selected = !card.selected
}

const isDragging = ref(false)
const draggedCards = ref(new Set<string>())

const onDragStart = (card: PokerCard) => {
  isDragging.value = true
  draggedCards.value.clear()
  toggleCard(card)
  draggedCards.value.add(card.id)
}

const onDragEnter = (card: PokerCard) => {
  if (isDragging.value && !draggedCards.value.has(card.id)) {
    toggleCard(card)
    draggedCards.value.add(card.id)
  }
}

const onTouchMove = (e: TouchEvent) => {
  if (!isDragging.value) return
  const touch = e.touches[0]
  const el = document.elementFromPoint(touch.clientX, touch.clientY)
  if (el) {
    const cardEl = el.closest('[data-card-id]')
    if (cardEl) {
      const id = cardEl.getAttribute('data-card-id')
      const card = myCards.value.find(c => c.id === id)
      if (card && !draggedCards.value.has(card.id)) {
        toggleCard(card)
        draggedCards.value.add(card.id)
      }
    }
  }
}

const onDragEnd = () => {
  isDragging.value = false
  draggedCards.value.clear()
}

onMounted(() => {
  window.addEventListener('mouseup', onDragEnd)
  window.addEventListener('touchend', onDragEnd)
})

onUnmounted(() => {
  window.removeEventListener('mouseup', onDragEnd)
  window.removeEventListener('touchend', onDragEnd)
})

const rankValues: Record<CardRank, number> = {
  '3': 3, '4': 4, '5': 5, '6': 6, '7': 7, '8': 8, '9': 9, '10': 10, 'J': 11, 'Q': 12, 'K': 13, 'A': 14, '2': 15, 'S': 16, 'B': 17
}

const checkValidPlay = (cards: PokerCard[]) => {
  if (cards.length === 0) return false
  if (cards.length === 1) return true
  if (cards.length === 2) {
    if ((cards[0].rank === 'S' && cards[1].rank === 'B') || (cards[0].rank === 'B' && cards[1].rank === 'S')) return true // 王炸
    return cards[0].rank === cards[1].rank // 对子
  }
  
  const counts: Record<string, number> = {}
  cards.forEach(c => { counts[c.rank] = (counts[c.rank] || 0) + 1 })
  const countsArr = Object.values(counts)
  const maxCount = Math.max(...countsArr)
  
  if (cards.length === 3) return maxCount === 3
  if (cards.length === 4) return maxCount === 4 || maxCount === 3 // 炸弹 或 三带一
  if (cards.length === 5 && maxCount === 3 && countsArr.includes(2)) return true // 三带一对
  
  // 顺子
  if (maxCount === 1 && cards.length >= 5) {
    const sorted = cards.map(c => rankValues[c.rank]).sort((a,b) => a-b)
    if (sorted[sorted.length-1] >= 15) return false // 顺子不能包含 2, S, B
    let isStraight = true
    for (let i = 1; i < sorted.length; i++) {
      if (sorted[i] !== sorted[i-1] + 1) { isStraight = false; break; }
    }
    if (isStraight) return true
  }
  
  // 连对
  if (maxCount === 2 && countsArr.every(c => c === 2) && cards.length >= 6) {
    const sorted = Object.keys(counts).map(r => rankValues[r as CardRank]).sort((a,b) => a-b)
    if (sorted[sorted.length-1] >= 15) return false
    let isStraight = true
    for (let i = 1; i < sorted.length; i++) {
      if (sorted[i] !== sorted[i-1] + 1) { isStraight = false; break; }
    }
    if (isStraight) return true
  }
  
  // 飞机
  if (maxCount === 3) {
    const threes = Object.keys(counts).filter(k => counts[k] === 3).map(r => rankValues[r as CardRank]).sort((a,b) => a-b)
    if (threes.length >= 2) {
      let isStraight = true
      if (threes[threes.length-1] >= 15) isStraight = false
      for (let i = 1; i < threes.length; i++) {
         if (threes[i] !== threes[i-1] + 1) isStraight = false
      }
      if (isStraight) {
        if (cards.length === threes.length * 3) return true // 飞机不带
        if (cards.length === threes.length * 4) return true // 飞机带单
        if (cards.length === threes.length * 5 && countsArr.filter(c => c === 2).length === threes.length) return true // 飞机带对
      }
    }
  }
  
  // 四带二
  if (maxCount === 4) {
    if (cards.length === 6) return true // 四带二单
    if (cards.length === 8 && countsArr.filter(c => c === 2).length === 2) return true // 四带二对
  }

  return false
}

const playCards = () => {
  const selected = myCards.value.filter(c => c.selected)
  if (selected.length === 0) {
    message.warning('请选择要出的牌')
    return
  }
  if (!checkValidPlay(selected)) {
    message.error('出牌不符合规则！')
    return
  }
  // Mock playing
  tableCards.value = selected.map(c => ({...c, selected: false}))
  myCards.value = myCards.value.filter(c => !c.selected)
  actionStatus.value = 'waiting'
  
  // Mock next turn
  setTimeout(() => {
    actionStatus.value = 'play'
  }, 3000)
}

const pass = () => {
  message.info('不出')
  actionStatus.value = 'waiting'
  setTimeout(() => {
    actionStatus.value = 'play'
  }, 3000)
}

const getSuitColor = (suit: CardSuit) => {
  if (suit === 'heart' || suit === 'diamond') return 'text-red-600'
  if (suit === 'joker') return 'text-purple-600'
  return 'text-slate-800'
}

const getSuitSymbol = (suit: CardSuit) => {
  switch (suit) {
    case 'spade': return '♠'
    case 'heart': return '♥'
    case 'club': return '♣'
    case 'diamond': return '♦'
    default: return ''
  }
}

const toggleSound = () => {
  soundEnabled.value = !soundEnabled.value
}
</script>

<template>
  <div class="relative w-screen h-screen overflow-hidden bg-slate-900 font-sans select-none flex flex-col">
    
    <!-- Top Navigation Bar -->
    <div class="h-12 bg-slate-900/80 backdrop-blur border-b border-slate-700/50 flex items-center justify-between px-4 z-50 shadow-sm relative">
      <div class="flex items-center gap-3">
        <n-button quaternary circle size="small" @click="phase === 'lobby' ? null : phase = 'lobby'">
          <template #icon><n-icon :component="ArrowLeft" class="text-slate-300" /></template>
        </n-button>
        <div class="flex items-center gap-2 text-slate-200 font-semibold tracking-wide">
          <span class="text-xl">♠</span>
          <span>局域网斗地主</span>
        </div>
      </div>
      <div class="flex items-center gap-2">
        <n-button quaternary circle size="small" @click="toggleSound">
          <template #icon>
            <n-icon :component="soundEnabled ? VolumeUpFilled : VolumeMuteFilled" class="text-slate-300" />
          </template>
        </n-button>
        <n-button quaternary circle size="small">
          <template #icon><n-icon :component="Settings" class="text-slate-300" /></template>
        </n-button>
      </div>
    </div>

    <!-- Phase: Lobby -->
    <div v-if="phase === 'lobby'" class="flex-1 flex items-center justify-center relative overflow-hidden bg-[radial-gradient(ellipse_at_top,_var(--tw-gradient-stops))] from-slate-800 via-slate-900 to-black">
      <!-- Decorative background cards -->
      <div class="absolute inset-0 opacity-10 pointer-events-none overflow-hidden">
        <div class="absolute top-10 left-10 text-[200px] text-red-500 rotate-[-15deg]">♥</div>
        <div class="absolute bottom-10 right-20 text-[250px] text-slate-500 rotate-[25deg]">♠</div>
        <div class="absolute top-1/2 left-1/3 text-[150px] text-red-500 rotate-[45deg]">♦</div>
      </div>

      <div class="w-[480px] bg-white/5 backdrop-blur-xl border border-white/10 rounded-3xl p-10 shadow-2xl relative z-10 flex flex-col gap-8">
        <div class="text-center space-y-2">
          <h1 class="text-4xl font-black bg-gradient-to-br from-yellow-300 via-yellow-500 to-orange-500 bg-clip-text text-transparent drop-shadow-md">
            欢乐斗地主
          </h1>
          <p class="text-slate-400 font-medium">局域网联机对战</p>
        </div>

        <div class="space-y-6">
          <div class="space-y-2">
            <label class="text-slate-300 text-sm font-semibold flex items-center gap-2">
              <n-icon :component="User" /> 你的昵称
            </label>
            <n-input v-model:value="myName" size="large" placeholder="输入昵称" class="bg-black/20" />
          </div>

          <div class="pt-4 flex flex-col gap-4">
            <n-button type="warning" size="large" block class="h-14 text-lg font-bold shadow-lg shadow-orange-500/20" @click="createRoom">
              <template #icon><n-icon :component="Wifi" /></template>
              创建房间 (作为房主)
            </n-button>
            
            <div class="relative flex items-center py-2">
              <div class="flex-grow border-t border-slate-600"></div>
              <span class="flex-shrink-0 mx-4 text-slate-500 text-sm">或</span>
              <div class="flex-grow border-t border-slate-600"></div>
            </div>

            <div class="flex gap-2">
              <n-input v-model:value="joinIp" size="large" placeholder="房主 IP 地址 (例如: 192.168.1.100)" class="flex-1 bg-black/20" />
              <n-button type="primary" size="large" class="px-8 shadow-lg shadow-blue-500/20" @click="joinRoom">
                加入
              </n-button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Phase: Room -->
    <div v-else-if="phase === 'room'" class="flex-1 flex items-center justify-center bg-[radial-gradient(ellipse_at_center,_var(--tw-gradient-stops))] from-blue-900/40 via-slate-900 to-black relative">
      <div class="w-[800px] bg-slate-800/80 backdrop-blur-md border border-slate-700 rounded-3xl p-8 shadow-2xl flex flex-col h-[600px]">
        <div class="flex justify-between items-center mb-8 border-b border-slate-700/50 pb-6">
          <div>
            <h2 class="text-2xl font-bold text-white flex items-center gap-3">
              <span class="w-3 h-3 rounded-full bg-green-500 animate-pulse"></span>
              游戏房间
            </h2>
            <p class="text-slate-400 mt-1">等待玩家加入... ({{ players.length }}/3)</p>
          </div>
          <div class="text-right">
            <div class="text-sm text-slate-400">本机 IP: <span class="text-yellow-400 font-mono select-all">192.168.31.55</span></div>
          </div>
        </div>

        <div class="flex-1 grid grid-cols-3 gap-6">
          <!-- Player Slots -->
          <div v-for="i in 3" :key="i" class="relative group">
            <div v-if="players[i-1]" class="h-full bg-slate-700/30 rounded-2xl border border-slate-600/50 p-6 flex flex-col items-center justify-center gap-4 transition-all hover:bg-slate-700/50">
              <div class="relative">
                <n-avatar :size="80" :src="players[i-1].avatar" round class="border-4 border-slate-600 shadow-xl bg-slate-800" />
                <div v-if="players[i-1].isHost" class="absolute -top-2 -right-2 bg-yellow-500 text-black text-xs font-bold px-2 py-1 rounded-full shadow-lg">
                  房主
                </div>
              </div>
              <div class="text-center">
                <div class="text-lg font-bold text-white">{{ players[i-1].name }}</div>
                <div class="text-sm text-green-400 mt-1 flex items-center justify-center gap-1">
                  <n-icon :component="Wifi" /> 已连接
                </div>
              </div>
            </div>
            <div v-else class="h-full bg-slate-800/30 rounded-2xl border border-dashed border-slate-600 p-6 flex flex-col items-center justify-center gap-4">
              <div class="w-20 h-20 rounded-full bg-slate-700/50 flex items-center justify-center text-slate-500">
                <n-icon :size="32" :component="User" />
              </div>
              <div class="text-slate-500 font-medium">等待加入...</div>
            </div>
          </div>
        </div>

        <div class="mt-8 flex justify-center pt-6 border-t border-slate-700/50">
          <n-button 
            v-if="players[0]?.isHost"
            type="warning" 
            size="large" 
            class="w-64 h-14 text-lg font-bold shadow-[0_0_20px_rgba(245,158,11,0.3)] transition-transform hover:scale-105"
            :disabled="players.length < 3"
            @click="startGame"
          >
            <template #icon><n-icon :component="PlayFilled" /></template>
            开始游戏
          </n-button>
          <div v-else class="text-slate-400 flex items-center gap-2">
            <n-icon :component="PlayFilled" class="animate-pulse" /> 等待房主开始游戏...
          </div>
        </div>
      </div>
    </div>

    <!-- Phase: Playing -->
    <div v-else-if="phase === 'playing'" class="flex-1 relative bg-[#0a3f23] overflow-hidden">
      <!-- Poker Table Felt Background -->
      <div class="absolute inset-0 opacity-40 bg-[url('https://www.transparenttextures.com/patterns/cubes.png')] mix-blend-overlay"></div>
      
      <!-- Table Lighting -->
      <div class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[800px] h-[500px] bg-green-500/20 blur-[100px] rounded-full pointer-events-none"></div>

      <!-- Top info (Base cards) -->
      <div class="absolute top-4 left-1/2 -translate-x-1/2 flex items-center gap-6 bg-black/40 backdrop-blur-sm px-6 py-3 rounded-full border border-white/10 z-10 shadow-xl">
        <div class="text-yellow-400 font-bold tracking-widest text-sm flex items-center gap-2">
          底牌
        </div>
        <div class="flex gap-2">
          <div v-for="card in topCards" :key="card.id" class="w-8 h-12 bg-white rounded flex items-center justify-center font-bold text-sm shadow-md" :class="getSuitColor(card.suit)">
            <div class="flex flex-col items-center leading-none">
              <span class="text-[10px]">{{ card.rank === 'S' ? '小' : (card.rank === 'B' ? '大' : card.rank) }}</span>
              <span v-if="card.suit !== 'joker'" class="text-[12px] -mt-0.5">{{ getSuitSymbol(card.suit) }}</span>
              <span v-else class="text-[10px] -mt-0.5">王</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Left Player -->
      <div class="absolute left-8 top-1/3 -translate-y-1/2 flex flex-col items-center gap-4 z-10">
        <div class="relative">
          <n-avatar :size="64" :src="players[1]?.avatar" round class="border-2 border-white/20 shadow-xl bg-slate-800" />
          <div class="absolute -bottom-2 left-1/2 -translate-x-1/2 bg-black/60 text-white text-xs px-2 py-0.5 rounded-full whitespace-nowrap border border-white/10 backdrop-blur">
            {{ players[1]?.name || 'Player 2' }}
          </div>
        </div>
        <div class="bg-black/40 backdrop-blur rounded-lg px-3 py-1.5 flex items-center gap-2 border border-white/10 shadow-lg">
          <div class="w-4 h-6 bg-slate-200 rounded-sm border border-slate-400 flex items-center justify-center">
            <div class="w-3 h-5 border border-slate-400 rounded-sm bg-slate-100"></div>
          </div>
          <span class="text-white font-bold text-lg">{{ players[1]?.cardCount || 17 }}</span>
        </div>
        <!-- Speech Bubble Mock -->
        <div class="absolute left-full ml-4 top-4 bg-white text-black font-bold px-4 py-2 rounded-xl rounded-tl-none shadow-xl whitespace-nowrap">
          不出
        </div>
      </div>

      <!-- Right Player -->
      <div class="absolute right-8 top-1/3 -translate-y-1/2 flex flex-col items-center gap-4 z-10">
        <div class="relative">
          <n-avatar :size="64" :src="players[2]?.avatar" round class="border-2 border-white/20 shadow-xl bg-slate-800" />
          <div class="absolute -bottom-2 left-1/2 -translate-x-1/2 bg-black/60 text-white text-xs px-2 py-0.5 rounded-full whitespace-nowrap border border-white/10 backdrop-blur">
            {{ players[2]?.name || 'Player 3' }}
          </div>
        </div>
        <div class="bg-black/40 backdrop-blur rounded-lg px-3 py-1.5 flex items-center gap-2 border border-white/10 shadow-lg">
          <span class="text-white font-bold text-lg">{{ players[2]?.cardCount || 17 }}</span>
          <div class="w-4 h-6 bg-slate-200 rounded-sm border border-slate-400 flex items-center justify-center">
            <div class="w-3 h-5 border border-slate-400 rounded-sm bg-slate-100"></div>
          </div>
        </div>
      </div>

      <!-- Center Table Cards (History) -->
      <div class="absolute top-[45%] left-1/2 -translate-x-1/2 -translate-y-1/2 z-0">
        <div class="flex relative -space-x-10 scale-125 origin-center transition-transform hover:scale-150 drop-shadow-2xl">
          <div v-for="(card, index) in tableCards" :key="card.id" 
               class="w-[85px] h-[120px] bg-white rounded-lg border-2 border-slate-200 shadow-xl flex flex-col p-1.5 relative bg-gradient-to-br from-white to-slate-50"
               :class="[getSuitColor(card.suit)]"
               :style="{ zIndex: index }">
            
            <div class="flex flex-col items-center absolute top-1 left-1.5">
              <span class="text-xl font-black leading-none" :class="{'text-purple-600': card.suit === 'joker', 'text-[14px]': card.suit === 'joker'}">
                {{ card.rank === 'S' ? '小' : (card.rank === 'B' ? '大' : card.rank) }}
              </span>
              <span v-if="card.suit !== 'joker'" class="text-lg leading-none mt-0.5">{{ getSuitSymbol(card.suit) }}</span>
              <span v-else class="text-[10px] font-black leading-none mt-0.5 text-purple-600">王</span>
            </div>

            <div class="flex flex-col items-center absolute bottom-1 right-1.5 rotate-180">
              <span class="text-xl font-black leading-none" :class="{'text-purple-600': card.suit === 'joker', 'text-[14px]': card.suit === 'joker'}">
                {{ card.rank === 'S' ? '小' : (card.rank === 'B' ? '大' : card.rank) }}
              </span>
              <span v-if="card.suit !== 'joker'" class="text-lg leading-none mt-0.5">{{ getSuitSymbol(card.suit) }}</span>
              <span v-else class="text-[10px] font-black leading-none mt-0.5 text-purple-600">王</span>
            </div>

            <div class="absolute inset-0 flex items-center justify-center opacity-10 pointer-events-none">
               <div v-if="card.suit !== 'joker'" class="text-5xl">{{ getSuitSymbol(card.suit) }}</div>
               <div v-else class="text-3xl font-black rotate-[-20deg]" :class="card.rank === 'B' ? 'text-red-500' : 'text-black'">
                 {{ card.rank === 'S' ? 'JOKER' : 'JOKER' }}
               </div>
            </div>
          </div>
        </div>
      </div>

      <!-- My Area -->
      <div class="absolute bottom-0 left-0 right-0 h-[280px] flex flex-col items-center z-20">
        
        <!-- Action Buttons -->
        <div class="h-16 flex items-center gap-4 mb-4" v-if="actionStatus === 'play'">
          <n-button strong secondary type="default" size="large" class="px-8 rounded-full text-lg shadow-lg font-bold bg-slate-800/80 text-white hover:bg-slate-700/80 border-slate-600" @click="pass">不出</n-button>
          <n-button strong secondary type="warning" size="large" class="px-8 rounded-full text-lg shadow-lg font-bold">提示</n-button>
          <n-button type="info" size="large" class="px-12 rounded-full text-lg shadow-[0_0_20px_rgba(14,165,233,0.4)] font-bold" @click="playCards">出牌</n-button>
        </div>
        <div class="h-16 flex items-center mb-4 text-white/70 font-bold tracking-widest text-lg bg-black/30 px-6 py-2 rounded-full backdrop-blur-sm border border-white/10" v-else-if="actionStatus === 'waiting'">
          等待其他玩家出牌...
        </div>
        <div v-else class="h-16 mb-4"></div>

        <!-- My Avatar Info -->
        <div class="absolute left-8 bottom-8 flex items-center gap-4 bg-black/30 p-2 pr-6 rounded-full backdrop-blur-sm border border-white/10 shadow-lg">
          <n-avatar :size="56" :src="players[0]?.avatar" round class="border-2 border-white/20 bg-slate-800" />
          <div>
            <div class="text-white font-bold text-sm">{{ players[0]?.name }}</div>
            <div class="text-yellow-400 text-xs font-bold mt-0.5">普通玩家</div>
          </div>
        </div>

        <!-- My Cards Hand -->
        <div class="relative flex justify-center px-10 max-w-full overflow-visible h-[150px]">
          <div class="flex -space-x-[50px] hover:-space-x-[35px] transition-all duration-300 items-end px-10" @mouseleave="isDragging = false">
            <div v-for="(card, index) in myCards" :key="card.id" 
                 :data-card-id="card.id"
                 @mousedown="onDragStart(card)"
                 @mouseenter="onDragEnter(card)"
                 @touchstart.prevent="onDragStart(card)"
                 @touchmove.prevent="onTouchMove"
                 class="w-[105px] h-[150px] bg-white rounded-xl border-2 border-slate-200 flex flex-col p-2 relative cursor-pointer select-none transition-all duration-200 shadow-[-2px_4px_10px_rgba(0,0,0,0.2)] hover:shadow-[-4px_10px_20px_rgba(0,0,0,0.4)]"
                 :class="[
                   getSuitColor(card.suit),
                   card.selected ? '-translate-y-6 shadow-[-4px_12px_24px_rgba(0,0,0,0.4)] border-blue-400' : 'hover:-translate-y-2'
                 ]"
                 :style="{ zIndex: index }">
              
              <div class="absolute inset-1 rounded-lg border border-slate-100/50 pointer-events-none"></div>

              <!-- Top Left -->
              <div class="flex flex-col items-center absolute top-1.5 left-1.5">
                <span class="text-2xl font-black leading-none" :class="{'text-purple-600': card.suit === 'joker', 'text-[18px]': card.suit === 'joker'}">
                  {{ card.rank === 'S' ? '小' : (card.rank === 'B' ? '大' : card.rank) }}
                </span>
                <span v-if="card.suit !== 'joker'" class="text-xl leading-none mt-0.5">{{ getSuitSymbol(card.suit) }}</span>
                <span v-else class="text-xs font-black leading-none mt-0.5 text-purple-600">王</span>
              </div>

              <!-- Bottom Right (Inverted) -->
              <div class="flex flex-col items-center absolute bottom-1.5 right-1.5 rotate-180">
                <span class="text-2xl font-black leading-none" :class="{'text-purple-600': card.suit === 'joker', 'text-[18px]': card.suit === 'joker'}">
                  {{ card.rank === 'S' ? '小' : (card.rank === 'B' ? '大' : card.rank) }}
                </span>
                <span v-if="card.suit !== 'joker'" class="text-xl leading-none mt-0.5">{{ getSuitSymbol(card.suit) }}</span>
                <span v-else class="text-xs font-black leading-none mt-0.5 text-purple-600">王</span>
              </div>

              <!-- Center Large Symbol -->
              <div class="absolute inset-0 flex items-center justify-center opacity-15 pointer-events-none">
                 <div v-if="card.suit !== 'joker'" class="text-6xl">{{ getSuitSymbol(card.suit) }}</div>
                 <div v-else class="text-4xl font-black rotate-[-20deg]" :class="card.rank === 'B' ? 'text-red-500' : 'text-black'">
                   {{ card.rank === 'S' ? 'JOKER' : 'JOKER' }}
                 </div>
              </div>
              
              <!-- Highlight Overlay -->
              <div v-if="card.selected" class="absolute inset-0 bg-blue-500/10 rounded-xl pointer-events-none mix-blend-multiply"></div>
            </div>
          </div>
        </div>
      </div>
    </div>

  </div>
</template>

<style scoped>
/* Any custom scoped styles */
</style>
