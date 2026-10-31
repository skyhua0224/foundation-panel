<template>
  <div class="ds5t-trigger">
    <div class="ds5t-trigger-modes" role="radiogroup" :aria-label="tr('trigger_preview.mode')">
      <button
        v-for="mode in modes"
        :key="mode"
        type="button"
        class="ds5t-trigger-mode"
        :class="{ selected: mode === selectedMode }"
        role="radio"
        :aria-checked="mode === selectedMode"
        @click="selectedMode = mode"
      >
        {{ tr('trigger_preview.' + mode) }}
      </button>
    </div>
    <div class="ds5t-trigger-charts">
      <figure v-for="side in sides" :key="side.id" class="ds5t-trigger-chart">
        <figcaption>{{ side.label }}</figcaption>
        <svg viewBox="0 0 100 100" role="img" :aria-label="side.label">
          <path class="ds5t-trigger-grid" d="M0 25H100M0 50H100M0 75H100M25 0V100M50 0V100M75 0V100" />
          <polyline class="ds5t-trigger-line" :class="'line-' + side.id" :points="path(side.strength)" />
        </svg>
      </figure>
    </div>
    <p class="ds5t-trigger-note">{{ tr('trigger_preview.note') }}</p>
  </div>
</template>

<script setup>
import { computed, ref } from 'vue'
import { useDualSenseText } from '../../composables/useDualSenseText.js'
import { triggerForce } from '../../composables/dualsenseCurves.js'

const props = defineProps({
  enabled: { type: Boolean, default: true },
  strength: { type: Number, default: 1 },
  leftStrength: { type: Number, default: 1 },
  rightStrength: { type: Number, default: 1 },
})

const { tr } = useDualSenseText()
const selectedMode = ref('feedback')
const modes = ['feedback', 'weapon', 'vibration', 'slope']
const sides = computed(() => [
  { id: 'left', label: tr('trigger_preview.left'), strength: props.leftStrength },
  { id: 'right', label: tr('trigger_preview.right'), strength: props.rightStrength },
])

const path = (sideStrength) => {
  const points = []
  for (let index = 0; index <= 64; ++index) {
    const input = index / 64
    const force = triggerForce(selectedMode.value, input, {
      enabled: props.enabled,
      strength: props.strength,
      sideStrength,
    })
    points.push(`${(input * 100).toFixed(2)},${(100 - force * 100).toFixed(2)}`)
  }
  return points.join(' ')
}
</script>

<style scoped>
.ds5t-trigger {
  display: grid;
  gap: 0.625rem;
}

.ds5t-trigger-modes {
  display: inline-flex;
  flex-wrap: wrap;
  gap: 2px;
  padding: 2px;
  border-radius: var(--ds5t-control-radius);
  background: var(--ds5t-segment-bg);
  width: fit-content;
  max-width: 100%;
}

.ds5t-trigger-mode {
  height: 1.625rem;
  padding: 0 0.625rem;
  border: 0;
  border-radius: calc(var(--ds5t-control-radius) - 2px);
  background: transparent;
  color: var(--ds5t-muted);
  font-size: 0.75rem;
  font-weight: 500;
  cursor: pointer;
}

.ds5t-trigger-mode.selected {
  background: var(--ds5t-accent);
  color: var(--ds5t-on-accent);
}

.ds5t-trigger-mode:focus-visible {
  outline: 2px solid var(--ds5t-accent);
  outline-offset: 2px;
}

.ds5t-trigger-charts {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.75rem;
}

.ds5t-trigger-chart {
  margin: 0;
}

.ds5t-trigger-chart figcaption {
  margin-bottom: 0.25rem;
  color: var(--ds5t-muted);
  font-size: 0.75rem;
}

.ds5t-trigger-chart svg {
  width: 100%;
  height: 5rem;
  border-radius: 0.5rem;
  background: var(--ds5t-sunken);
  box-shadow: inset 0 0 0 1px var(--ds5t-border);
}

.ds5t-trigger-grid {
  fill: none;
  stroke: var(--ds5t-sep);
  stroke-width: 0.6;
}

.ds5t-trigger-line {
  fill: none;
  stroke: var(--ds5t-accent);
  stroke-linecap: round;
  stroke-width: 2;
}

.ds5t-trigger-line.line-right {
  stroke: var(--ds5t-muted);
}

.ds5t-trigger-note {
  margin: 0;
  color: var(--ds5t-muted);
  font-size: 0.75rem;
}
</style>
