<template>
  <MPage :title="text.title">
    <div class="m-stream-timer" :style="{ '--stream-timer-scale': scale }" role="timer" aria-live="off">
      <div class="m-stream-timer__face">
        <MIcon name="schedule" size="32" />
        <strong>{{ time }}</strong>
        <span>{{ date }}</span>
      </div>
    </div>
    <MGroup>
      <MItem icon="schedule" :title="text.scale" :value="`${Math.round(scale * 100)}%`" />
      <MItem stack :title="text.adjust">
        <template #control>
          <div class="m-stream-timer__controls">
            <MButton variant="tonal" icon="remove" :aria-label="text.decrease" :disabled="scale <= MIN_SCALE" @click="scale = Math.max(MIN_SCALE, scale - STEP)" />
            <MButton variant="tonal" icon="add" :aria-label="text.increase" :disabled="scale >= MAX_SCALE" @click="scale = Math.min(MAX_SCALE, scale + STEP)" />
            <MButton variant="text" icon="restart_alt" :aria-label="text.reset" @click="scale = 1" />
          </div>
        </template>
      </MItem>
    </MGroup>
  </MPage>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { MButton, MGroup, MIcon, MItem, MPage } from 'foundation-ui/vue'
import { useI18n } from '../../desktop/i18n/index.js'

const MIN_SCALE = 0.6
const MAX_SCALE = 1.4
const STEP = 0.2
const { t, locale } = useI18n()
const text = computed(() => ({ title: t.value.sidebar.streamTimer, scale: t.value.timer.scale, adjust: t.value.timer.adjust, decrease: t.value.timer.decrease, increase: t.value.timer.increase, reset: t.value.timer.reset }))
const now = ref(new Date())
const scale = ref(1)
let ticker = 0
const time = computed(() => new Intl.DateTimeFormat(locale.value === 'zh' ? 'zh-CN' : 'en', { hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false }).format(now.value))
const date = computed(() => new Intl.DateTimeFormat(locale.value === 'zh' ? 'zh-CN' : 'en', { weekday: 'long', year: 'numeric', month: 'long', day: 'numeric' }).format(now.value))
onMounted(() => { ticker = window.setInterval(() => { now.value = new Date() }, 100) })
onBeforeUnmount(() => window.clearInterval(ticker))
</script>

<style scoped>
.m-stream-timer { display: grid; place-items: center; min-height: min(46vh, 420px); padding: 24px; }
.m-stream-timer__face { display: grid; justify-items: center; gap: 10px; transform: scale(var(--stream-timer-scale)); transform-origin: center; color: var(--md-sys-color-on-surface); }
.m-stream-timer__face strong { font: var(--m-display-medium); font-variant-numeric: tabular-nums; letter-spacing: 0; }
.m-stream-timer__face span { font: var(--m-title-medium); color: var(--md-sys-color-on-surface-variant); }
.m-stream-timer__controls { display: flex; gap: 8px; }
</style>
