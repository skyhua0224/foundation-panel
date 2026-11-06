<template>
  <Teleport to="body">
    <Transition name="back-ring">
      <div v-if="visible" ref="ringRef" class="back-hold-ring" popover="manual" aria-hidden="true">
        <svg viewBox="0 0 48 48">
          <circle class="ring-track" cx="24" cy="24" r="20" />
          <circle
            class="ring-fill"
            cx="24"
            cy="24"
            r="20"
            :stroke-dasharray="CIRCUMFERENCE"
            :stroke-dashoffset="CIRCUMFERENCE * (1 - progress)"
          />
        </svg>
        <span class="ring-label">{{ t.gamepadLegend.backRootHint }}</span>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup>
import { computed, nextTick, ref, watch } from 'vue'
import { backHoldProgress } from '../composables/useGamepad.js'
import { useI18n } from '../i18n/index.js'

const { t } = useI18n()

const CIRCUMFERENCE = 2 * Math.PI * 20
const props = defineProps({
  progress: { type: Number, default: 0 },
})

const visible = computed(() => props.progress > 0.06)

// popover 进顶层：长按 B 时即使有对话框 / 屏幕键盘开着，环也画在最上面
const ringRef = ref(null)
watch(visible, async (on) => {
  if (!on) return
  await nextTick()
  try { ringRef.value?.showPopover?.() } catch { /* 已经打开，或没有顶层 */ }
})
</script>

<style lang="less" scoped>
.back-hold-ring {
  position: fixed;
  inset: auto;
  left: 50%;
  bottom: 22vh;
  transform: translateX(-50%);
  margin: 0;
  padding: 0;
  border: 0;
  background: transparent;
  overflow: visible;
  z-index: 22000;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  pointer-events: none;

  svg {
    width: 88px;
    height: 88px;
    padding: 10px;
    border-radius: 50%;
    background: var(--md-sys-color-surface-container-high);
    box-shadow: var(--d-shadow-3);
    transform: rotate(-90deg);
  }

  .ring-track,
  .ring-fill {
    fill: none;
    stroke-width: 4;
    stroke-linecap: round;
  }

  .ring-track {
    stroke: var(--md-sys-color-outline-variant);
  }

  .ring-fill {
    stroke: var(--md-sys-color-primary);
    transition: stroke-dashoffset 16ms linear;
  }

  .ring-label {
    font: 500 16px/24px var(--md-ref-typeface-brand, system-ui);
    color: var(--md-sys-color-on-surface);
    background: var(--md-sys-color-surface-container-high);
    padding: 6px 18px;
    border-radius: 9999px;
    box-shadow: var(--d-shadow-1);
  }
}

.back-ring-enter-active,
.back-ring-leave-active {
  transition:
    opacity var(--m-spring-fast-effects-duration, 120ms) var(--m-spring-fast-effects, ease),
    transform var(--m-spring-fast-spatial-duration, 120ms) var(--m-spring-fast-spatial, ease);
}

.back-ring-enter-from,
.back-ring-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(6px);
}
</style>
