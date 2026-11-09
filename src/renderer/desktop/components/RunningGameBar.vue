<template>
  <Transition name="running-bar">
    <div v-if="game" class="running-bar" role="status">
      <span class="running-pulse" aria-hidden="true"></span>

      <div class="running-info">
        <span class="running-label">{{ t.gameSession.nowPlaying }}</span>
        <span class="running-name" data-redact="text" :title="game.appName">{{ game.appName }}</span>
      </div>

      <div class="running-meta">
        <MChip class="running-elapsed" icon="schedule">{{ formatDuration(elapsed) }}</MChip>
        <MChip v-if="totalLabel" class="running-total">{{ totalLabel }}</MChip>
        <MChip v-if="game.adopted" class="running-adopted" tone="tertiary" :title="t.gameSession.adoptedHint">
          {{ t.gameSession.adopted }}
        </MChip>
      </div>

      <div class="running-actions">
        <MButton size="lg" icon="play_arrow" data-focusable data-focus-key="running-resume" @click="emit('resume')">
          {{ t.gameSession.backToGame }}
        </MButton>
        <MButton variant="danger" size="lg" icon="stop_circle" data-focusable data-focus-key="running-stop" @click="emit('stop')">
          {{ t.gameSession.stopRunning }}
        </MButton>
      </div>
    </div>
  </Transition>
</template>

<script setup>
import { computed } from 'vue'
import { MButton, MChip } from 'foundation-ui/vue'
import { formatDuration } from '../composables/useGameSession.js'
import { useI18n } from '../i18n/index.js'

const { t } = useI18n()

const props = defineProps({
  game: { type: Object, default: null },
  elapsed: { type: Number, default: 0 },
  stat: { type: Object, default: null },
})

const emit = defineEmits(['resume', 'stop'])

const totalLabel = computed(() => {
  const total = Number(props.stat?.totalSeconds) || 0
  // 本局时长还没结算进统计，所以这里加上去，显示的才是真实累计
  const combined = total + (Number(props.elapsed) || 0)
  if (combined < 60) return ''
  return t.value.gameSession.totalPlaytime.replace('{duration}', formatDuration(combined))
})
</script>
<style lang="less" scoped>
// 悬浮在底部的「正在运行」条：surface-container-high 的大圆角药丸，抬升 3
.running-bar {
  display: flex;
  align-items: center;
  gap: 24px;
  padding: 14px 16px 14px 28px;
  border-radius: 9999px;
  background: var(--md-sys-color-surface-container-high);
  color: var(--md-sys-color-on-surface);
  box-shadow: var(--d-shadow-3);
}

.running-pulse {
  flex: none;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--md-sys-color-success, var(--md-sys-color-primary));
  animation: running-pulse 1.8s ease-in-out infinite;
}

@keyframes running-pulse {
  0%,
  100% {
    box-shadow: 0 0 0 0 color-mix(in srgb, var(--md-sys-color-success, var(--md-sys-color-primary)) 50%, transparent);
  }
  50% {
    box-shadow: 0 0 0 10px transparent;
  }
}

.running-info {
  display: flex;
  flex-direction: column;
  min-width: 0;
  max-width: 320px;

  .running-label {
    font: 500 13px/18px var(--md-ref-typeface-brand, system-ui);
    color: var(--md-sys-color-on-surface-variant);
  }

  .running-name {
    font: 700 20px/28px var(--md-ref-typeface-brand, system-ui);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}

.running-meta {
  display: flex;
  gap: 8px;
}

.running-actions {
  display: flex;
  gap: 10px;
}

.running-bar-enter-active {
  transition: opacity var(--m-spring-default-effects-duration, 250ms) var(--m-spring-default-effects, ease),
    transform var(--m-spring-default-spatial-duration, 400ms) var(--m-spring-default-spatial, ease);
}
.running-bar-leave-active {
  transition: opacity var(--m-spring-fast-effects-duration, 150ms) var(--m-spring-fast-effects, ease),
    transform var(--m-spring-fast-spatial-duration, 200ms) var(--m-spring-fast-spatial, ease);
}
.running-bar-enter-from,
.running-bar-leave-to {
  opacity: 0;
  transform: translateY(24px);
}
</style>
