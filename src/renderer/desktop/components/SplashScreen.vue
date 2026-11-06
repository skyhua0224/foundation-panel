<template>
  <Transition name="splash" @after-leave="$emit('done')">
    <div v-if="visible" class="splash-screen m-app" role="status" aria-live="polite" aria-label="Foundation Sunshine">
      <div class="splash-content" :class="{ reveal: phase >= 1, expand: phase >= 2 }">
        <div class="splash-logo">
          <span class="logo-shape" aria-hidden="true"></span>
          <img class="logo-icon" :src="sunshineIcon" alt="" />
        </div>

        <div class="splash-title">
          <span class="title-main">Foundation</span>
          <span class="title-sub">Sunshine</span>
        </div>

        <MProgress class="splash-progress" :value="phase >= 3 ? 1 : phase >= 2 ? 0.7 : phase >= 1 ? 0.35 : 0.08" flat label="" />
      </div>
    </div>
  </Transition>
</template>

<script setup>
import { ref, watch, onMounted, onUnmounted } from 'vue'
import { MProgress } from 'foundation-ui/vue'
import sunshineIcon from '../../../assets/sunshine.ico'

const props = defineProps({
  visible: { type: Boolean, default: true },
})

const emit = defineEmits(['done'])

const phase = ref(0)
let timers = []

// ===== 手柄振动 =====
// 缓存通过 gamepadconnected 事件发现的手柄索引
const connectedGamepadIndices = new Set()

function onGamepadConnected(e) {
  connectedGamepadIndices.add(e.gamepad.index)
  console.log('[SplashScreen] gamepad connected:', e.gamepad.index, e.gamepad.id)
}

function onGamepadDisconnected(e) {
  connectedGamepadIndices.delete(e.gamepad.index)
}

function vibrateGamepad(strong = 0, weak = 0, duration = 100) {
  try {
    const gamepads = navigator.getGamepads()
    let vibrated = false

    for (const gp of gamepads) {
      if (!gp) continue

      // 尝试标准 vibrationActuator
      if (gp.vibrationActuator) {
        gp.vibrationActuator.playEffect('dual-rumble', {
          startDelay: 0,
          duration,
          weakMagnitude: weak,
          strongMagnitude: strong,
        }).then(() => {
          console.log('[SplashScreen] vibration OK, gamepad:', gp.index)
        }).catch((err) => {
          console.warn('[SplashScreen] vibration failed:', err)
        })
        vibrated = true
      }
      // 尝试非标准 hapticActuators (Chrome fallback)
      else if (gp.hapticActuators?.length > 0) {
        gp.hapticActuators[0].pulse(strong, duration)
        vibrated = true
      }
    }

    if (!vibrated) {
      console.warn('[SplashScreen] no vibration-capable gamepad found.',
        'Connected indices:', [...connectedGamepadIndices],
        'getGamepads result:', gamepads.length,
        'non-null count:', [...gamepads].filter(Boolean).length)
    }
  } catch (err) {
    console.warn('[SplashScreen] vibrateGamepad error:', err)
  }
}

// ===== 音效系统 (Web Audio API 合成) =====
let audioCtx = null

function getAudioCtx() {
  if (!audioCtx) {
    audioCtx = new (window.AudioContext || window.webkitAudioContext)()
  }
  // 确保 AudioContext 处于运行状态
  if (audioCtx.state === 'suspended') {
    audioCtx.resume()
  }
  return audioCtx
}

/**
 * Phase 1: Logo 出现 — 可爱的弹跳 "叮~"
 */
function playSoundReveal() {
  const ctx = getAudioCtx()
  const now = ctx.currentTime

  // 清脆的高音 "叮" (类似音乐盒)
  const bell = ctx.createOscillator()
  bell.type = 'sine'
  bell.frequency.setValueAtTime(1318.5, now) // E6
  bell.frequency.exponentialRampToValueAtTime(1174.7, now + 0.3) // 轻微下滑，弹跳感

  const bellGain = ctx.createGain()
  bellGain.gain.setValueAtTime(0, now)
  bellGain.gain.linearRampToValueAtTime(0.25, now + 0.01)
  bellGain.gain.exponentialRampToValueAtTime(0.1, now + 0.15)
  bellGain.gain.exponentialRampToValueAtTime(0.01, now + 0.6)

  // 泛音层 (八度+五度 = 更亮更甜)
  const harm = ctx.createOscillator()
  harm.type = 'sine'
  harm.frequency.setValueAtTime(2637, now) // E7
  const harmGain = ctx.createGain()
  harmGain.gain.setValueAtTime(0, now)
  harmGain.gain.linearRampToValueAtTime(0.08, now + 0.01)
  harmGain.gain.exponentialRampToValueAtTime(0.01, now + 0.3)

  // 柔和的"噗"底音
  const pop = ctx.createOscillator()
  pop.type = 'sine'
  pop.frequency.setValueAtTime(300, now)
  pop.frequency.exponentialRampToValueAtTime(150, now + 0.08)

  const popGain = ctx.createGain()
  popGain.gain.setValueAtTime(0.2, now)
  popGain.gain.exponentialRampToValueAtTime(0.01, now + 0.12)

  bell.connect(bellGain).connect(ctx.destination)
  harm.connect(harmGain).connect(ctx.destination)
  pop.connect(popGain).connect(ctx.destination)

  bell.start(now); bell.stop(now + 0.7)
  harm.start(now); harm.stop(now + 0.4)
  pop.start(now); pop.stop(now + 0.15)
}

/**
 * Phase 2: 标题展开 — 暖甜的上升琶音 "叮咚叮~"
 */
function playSoundExpand() {
  const ctx = getAudioCtx()
  const now = ctx.currentTime

  // 快速上行琶音: C6 → E6 → G6 → C7
  const arpNotes = [1046.5, 1318.5, 1568, 2093]
  const spacing = 0.08

  arpNotes.forEach((freq, i) => {
    const t = now + i * spacing
    const osc = ctx.createOscillator()
    osc.type = 'sine'
    osc.frequency.setValueAtTime(freq, t)

    const gain = ctx.createGain()
    gain.gain.setValueAtTime(0, t)
    gain.gain.linearRampToValueAtTime(0.2, t + 0.01)
    gain.gain.exponentialRampToValueAtTime(0.06, t + 0.15)
    gain.gain.exponentialRampToValueAtTime(0.01, t + 0.5)

    osc.connect(gain).connect(ctx.destination)
    osc.start(t)
    osc.stop(t + 0.55)
  })

  // 琶音结束后的柔和 pad 余韵 (C 大三和弦)
  const padStart = now + arpNotes.length * spacing
  const padNotes = [523.25, 659.25, 783.99] // C5, E5, G5
  padNotes.forEach(freq => {
    const osc = ctx.createOscillator()
    osc.type = 'triangle'
    osc.frequency.setValueAtTime(freq, padStart)

    const gain = ctx.createGain()
    gain.gain.setValueAtTime(0, padStart)
    gain.gain.linearRampToValueAtTime(0.08, padStart + 0.1)
    gain.gain.exponentialRampToValueAtTime(0.01, padStart + 0.8)

    osc.connect(gain).connect(ctx.destination)
    osc.start(padStart)
    osc.stop(padStart + 0.9)
  })
}

/**
 * Phase 3: 完成 — 欢快的双音确认 "叮叮♪"
 */
function playSoundFinish() {
  const ctx = getAudioCtx()
  const now = ctx.currentTime

  // 两声活泼短音 (像收到奖励)
  const notes = [
    { freq: 1568, t: 0 },      // G6
    { freq: 2093, t: 0.1 },    // C7 (高八度，更欢快)
  ]

  notes.forEach(({ freq, t }) => {
    const osc = ctx.createOscillator()
    osc.type = 'sine'
    osc.frequency.setValueAtTime(freq, now + t)

    const gain = ctx.createGain()
    gain.gain.setValueAtTime(0, now + t)
    gain.gain.linearRampToValueAtTime(0.22, now + t + 0.01)
    gain.gain.exponentialRampToValueAtTime(0.01, now + t + 0.35)

    // 泛音
    const harm = ctx.createOscillator()
    harm.type = 'sine'
    harm.frequency.setValueAtTime(freq * 2, now + t)
    const harmGain = ctx.createGain()
    harmGain.gain.setValueAtTime(0, now + t)
    harmGain.gain.linearRampToValueAtTime(0.06, now + t + 0.01)
    harmGain.gain.exponentialRampToValueAtTime(0.01, now + t + 0.2)

    osc.connect(gain).connect(ctx.destination)
    harm.connect(harmGain).connect(ctx.destination)
    osc.start(now + t); osc.stop(now + t + 0.4)
    harm.start(now + t); harm.stop(now + t + 0.25)
  })

  // 轻柔尾韵 "嗡~"
  const hum = ctx.createOscillator()
  hum.type = 'triangle'
  hum.frequency.setValueAtTime(523.25, now + 0.15) // C5

  const humGain = ctx.createGain()
  humGain.gain.setValueAtTime(0, now + 0.15)
  humGain.gain.linearRampToValueAtTime(0.06, now + 0.25)
  humGain.gain.exponentialRampToValueAtTime(0.01, now + 0.7)

  hum.connect(humGain).connect(ctx.destination)
  hum.start(now + 0.15)
  hum.stop(now + 0.8)
}

function startAnimation() {
  phase.value = 0

  // Phase 1: Logo 出现 + 轻弹振动 + 音乐盒 "叮~"
  timers.push(setTimeout(() => {
    phase.value = 1
    vibrateGamepad(0.2, 0.5, 120)
    playSoundReveal()
  }, 300))

  // Phase 2: 标题展开 + 双脉冲振动 + 上行琶音 "叮咚叮~"
  timers.push(setTimeout(() => {
    phase.value = 2
    vibrateGamepad(0.3, 0.6, 150)
    playSoundExpand()
    // 第二下短脉冲
    setTimeout(() => vibrateGamepad(0.15, 0.4, 80), 160)
  }, 1000))

  // Phase 3: 完成 + 欢快双振 + 确认 "叮叮♪"
  timers.push(setTimeout(() => {
    phase.value = 3
    vibrateGamepad(0.25, 0.5, 100)
    setTimeout(() => vibrateGamepad(0.35, 0.6, 120), 110)
    playSoundFinish()
  }, 2200))

  timers.push(setTimeout(() => {
    emit('done')
  }, 2800))
}

onMounted(() => {
  window.addEventListener('gamepadconnected', onGamepadConnected)
  window.addEventListener('gamepaddisconnected', onGamepadDisconnected)

  // 检查已经连接的手柄（页面加载前已插入的手柄）
  try {
    const gamepads = navigator.getGamepads()
    for (const gp of gamepads) {
      if (gp) connectedGamepadIndices.add(gp.index)
    }
  } catch { /* ignore */ }

  if (props.visible) startAnimation()
})

watch(() => props.visible, (val) => {
  if (val) {
    startAnimation()
  } else {
    cleanup()
  }
})

function cleanup() {
  timers.forEach(clearTimeout)
  timers = []
}

onUnmounted(() => {
  cleanup()
  window.removeEventListener('gamepadconnected', onGamepadConnected)
  window.removeEventListener('gamepaddisconnected', onGamepadDisconnected)
  // 关闭 AudioContext 释放资源
  if (audioCtx) {
    audioCtx.close().catch(() => {})
    audioCtx = null
  }
})
</script>
<style lang="less" scoped>
// 启动页：surface 底，一枚缓慢旋转的 primary-container 形状托着图标，标题随弹簧进场，下方进度条按阶段前进
.splash-screen {
  position: fixed;
  inset: 0;
  z-index: 40000;
  display: grid;
  place-items: center;
  min-height: 0;
  // .m-app 给了页面底色变量（壁纸时透明）：启动页必须不透明
  background: var(--md-sys-color-surface) !important;
  color: var(--md-sys-color-on-surface);
}

.splash-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 32px;
  width: min(420px, 70vw);
}

.splash-logo {
  position: relative;
  display: grid;
  place-items: center;
  width: 200px;
  height: 200px;
  opacity: 0;
  transform: scale(0.6);
  transition: opacity var(--m-spring-default-effects-duration, 300ms) var(--m-spring-default-effects, ease),
    transform var(--m-spring-slow-spatial-duration, 600ms) var(--m-spring-slow-spatial, ease);

  .logo-shape {
    position: absolute;
    inset: 0;
    background: var(--md-sys-color-primary-container);
    clip-path: polygon(50% 0%, 61% 8%, 75% 6%, 82% 18%, 94% 25%, 92% 39%, 100% 50%, 92% 61%, 94% 75%, 82% 82%, 75% 94%, 61% 92%, 50% 100%, 39% 92%, 25% 94%, 18% 82%, 6% 75%, 8% 61%, 0% 50%, 8% 39%, 6% 25%, 18% 18%, 25% 6%, 39% 8%);
    animation: splash-turn 14s linear infinite;
  }

  .logo-icon {
    position: relative;
    width: 96px;
    height: 96px;
    object-fit: contain;
  }
}

@keyframes splash-turn {
  to {
    transform: rotate(360deg);
  }
}

.splash-title {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  opacity: 0;
  transform: translateY(16px);
  transition: opacity var(--m-spring-default-effects-duration, 300ms) var(--m-spring-default-effects, ease),
    transform var(--m-spring-default-spatial-duration, 500ms) var(--m-spring-default-spatial, ease);

  .title-main {
    font: 700 56px/64px var(--md-ref-typeface-brand, system-ui);
    letter-spacing: -1px;
  }

  .title-sub {
    font: 600 18px/24px var(--md-ref-typeface-brand, system-ui);
    letter-spacing: 6px;
    text-transform: uppercase;
    color: var(--md-sys-color-primary);
  }
}

.splash-progress {
  width: 100%;
  opacity: 0;
  transition: opacity var(--m-spring-default-effects-duration, 300ms) var(--m-spring-default-effects, ease);
}

.reveal .splash-logo {
  opacity: 1;
  transform: scale(1);
}

.expand .splash-title,
.expand .splash-progress {
  opacity: 1;
  transform: none;
}

.splash-leave-active {
  transition: opacity var(--m-spring-default-effects-duration, 400ms) var(--m-spring-default-effects, ease);
}

.splash-leave-to {
  opacity: 0;
}
</style>
