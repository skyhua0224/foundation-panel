<template>
  <MPage :title="text.title">
    <div class="m-stream-timer" :style="{ '--stream-timer-scale': scale }" role="timer" aria-live="off" :aria-label="`${date} ${time}`">
      <div class="m-stream-timer__face">
        <MClockDial :at="at" :size="260" />
        <div class="m-stream-timer__read">
          <strong>{{ time }}</strong>
          <span>{{ date }}</span>
        </div>
      </div>
    </div>
    <MGroup>
      <MSliderItem v-model="percent" icon="aspect_ratio" :title="text.scale" :min="MIN_PERCENT" :max="MAX_PERCENT" :step="STEP_PERCENT" :decimals="0" unit="%" size="m" icon-start="photo_size_select_small" icon-end="photo_size_select_large" />
      <MItem action icon="restart_alt" :title="text.reset" :disabled="percent === 100" @click="percent = 100" />
    </MGroup>
  </MPage>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { MClockDial, MGroup, MItem, MPage, MSliderItem } from 'foundation-ui/vue'
import { useI18n } from '../../desktop/i18n/index.js'

// 串流计时器: a Material clock (dial, time, date) whose size the person sets. The dial sweeps on animation frames while the page is visible; with 动画 减少 it ticks once a second.
const MIN_PERCENT = 60
const MAX_PERCENT = 140
const STEP_PERCENT = 20
const { t, locale } = useI18n()
const text = computed(() => ({ title: t.value.sidebar.streamTimer, scale: t.value.timer.scale, reset: t.value.timer.reset }))
const at = ref(Date.now())
const percent = ref(100)
const scale = computed(() => percent.value / 100)
let raf = 0
const tick = () => { at.value = Date.now(); raf = requestAnimationFrame(tick) }
const loc = computed(() => (locale.value === 'zh' ? 'zh-CN' : 'en'))
const time = computed(() => new Intl.DateTimeFormat(loc.value, { hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false }).format(at.value))
const date = computed(() => new Intl.DateTimeFormat(loc.value, { weekday: 'long', year: 'numeric', month: 'long', day: 'numeric' }).format(at.value))
const onVisible = () => { cancelAnimationFrame(raf); raf = 0; if (!document.hidden) raf = requestAnimationFrame(tick) }
onMounted(() => { raf = requestAnimationFrame(tick); document.addEventListener('visibilitychange', onVisible) })
onBeforeUnmount(() => { cancelAnimationFrame(raf); document.removeEventListener('visibilitychange', onVisible) })
</script>

<style scoped>
.m-stream-timer { display: grid; place-items: center; min-height: min(52vh, 460px); padding: 24px; overflow: hidden; border-radius: var(--md-sys-shape-corner-extra-large); background: var(--md-sys-color-surface-container); }
.m-stream-timer__face { display: flex; flex-wrap: wrap; align-items: center; justify-content: center; gap: 24px 40px; transform: scale(var(--stream-timer-scale)); transform-origin: center; color: var(--md-sys-color-on-surface); }
.m-stream-timer__read { display: grid; gap: 4px; justify-items: start; }
.m-stream-timer__read strong { font: var(--m-display-large); font-variant-numeric: tabular-nums; letter-spacing: 0; }
.m-stream-timer__read span { font: var(--m-title-medium); color: var(--md-sys-color-on-surface-variant); }
@media (prefers-reduced-motion: reduce) { .m-stream-timer { min-height: auto; } }
</style>
