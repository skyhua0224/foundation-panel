<template>
  <div class="ds5t-curve">
    <div class="ds5t-curve-options" role="radiogroup" :aria-label="label">
      <button
        v-for="option in curves"
        :key="option"
        type="button"
        class="ds5t-curve-option"
        :class="{ selected: option === modelValue }"
        role="radio"
        :aria-checked="option === modelValue"
        :disabled="disabled"
        @click="$emit('update:modelValue', option)"
      >
        {{ tr('curve.' + option) }}
      </button>
    </div>
    <svg class="ds5t-curve-chart" viewBox="0 0 100 100" role="img" :aria-label="label">
      <path class="ds5t-curve-grid" d="M0 25H100M0 50H100M0 75H100M25 0V100M50 0V100M75 0V100" />
      <path class="ds5t-curve-ghost" d="M0 100L100 0" />
      <polyline class="ds5t-curve-line" :points="points" />
      <circle class="ds5t-curve-end" cx="100" cy="0" r="2" />
    </svg>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { useDualSenseText } from '../../composables/useDualSenseText.js'
import { curveShape } from '../../composables/dualsenseCurves.js'

const props = defineProps({
  modelValue: { type: String, default: 'linear' },
  label: { type: String, default: '' },
  disabled: { type: Boolean, default: false },
})

defineEmits(['update:modelValue'])
const { tr } = useDualSenseText()

const curves = ['linear', 'enhanced', 'quadratic', 'cubic', 'easeOutQuad', 'easeOutCubic']

const points = computed(() => {
  const result = []
  for (let index = 0; index <= 64; ++index) {
    const input = index / 64
    result.push(`${(input * 100).toFixed(2)},${(100 - curveShape(props.modelValue, input) * 100).toFixed(2)}`)
  }
  return result.join(' ')
})
</script>

<style scoped>
.ds5t-curve {
  display: grid;
  gap: 0.5rem;
  width: 100%;
}

.ds5t-curve-options {
  display: inline-flex;
  flex-wrap: wrap;
  gap: 2px;
  padding: 2px;
  border-radius: var(--ds5t-control-radius);
  background: var(--ds5t-segment-bg);
  width: fit-content;
  max-width: 100%;
}

.ds5t-curve-option {
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

.ds5t-curve-option.selected {
  background: var(--ds5t-accent);
  color: var(--ds5t-on-accent);
}

.ds5t-curve-option:focus-visible {
  outline: 2px solid var(--ds5t-accent);
  outline-offset: 2px;
}

.ds5t-curve-option:disabled {
  opacity: 0.6;
}

.ds5t-curve-chart {
  width: 100%;
  max-width: 14rem;
  height: 5.5rem;
  border-radius: 0.5rem;
  background: var(--ds5t-sunken);
  box-shadow: inset 0 0 0 1px var(--ds5t-border);
}

.ds5t-curve-grid {
  fill: none;
  stroke: var(--ds5t-sep);
  stroke-width: 0.6;
}

.ds5t-curve-ghost {
  fill: none;
  stroke: var(--ds5t-muted);
  stroke-dasharray: 2 2;
  stroke-width: 0.8;
  opacity: 0.5;
}

.ds5t-curve-line {
  fill: none;
  stroke: var(--ds5t-accent);
  stroke-linecap: round;
  stroke-width: 2;
}

.ds5t-curve-end {
  fill: var(--ds5t-accent);
}
</style>
