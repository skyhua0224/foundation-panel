<!--
  One setting row: label, control, and a reset button that only shows once the
  value differs from the host's default. Sliders keep their value readout at the
  end of the row. The control kind follows the host descriptor (slider, toggle,
  segmented, menu, curve, chips); slider semantics stay on the native range input.
-->
<template>
  <div
    class="ds5t-row"
    :class="['kind-' + kind, { 'is-disabled': disabled, 'is-modified': modified }]"
    :title="disabled ? tr('pending.reason') : null"
  >
    <label :id="labelId" :for="hasInput ? fieldId : null" class="ds5t-row-label">
      <span>{{ fieldLabel }}</span>
      <span v-if="descriptor.expert" class="ds5t-tag">{{ tr('expert') }}</span>
    </label>

    <div class="ds5t-row-control">
      <div v-if="kind === 'slider'" class="ds5t-slider">
        <input
          :id="fieldId"
          class="ds5t-range"
          type="range"
          :min="sliderMin"
          :max="sliderMax"
          :step="sliderStep"
          :value="sliderValue"
          :style="{ '--p': sliderPercent + '%' }"
          :disabled="disabled"
          @input="onSliderInput"
          @dblclick="$emit('reset')"
        />
        <output class="ds5t-value" :for="fieldId">{{ displayValue }}</output>
      </div>

      <input
        v-else-if="kind === 'toggle'"
        :id="fieldId"
        class="ds5t-switch"
        type="checkbox"
        role="switch"
        :checked="modelValue === true"
        :disabled="disabled"
        @change="$emit('update:modelValue', $event.target.checked)"
      />

      <div v-else-if="kind === 'segmented'" class="ds5t-segmented" role="radiogroup" :aria-labelledby="labelId">
        <button
          v-for="option in descriptor.enum_names"
          :key="option"
          type="button"
          class="ds5t-segment"
          :class="{ selected: isSelected(option) }"
          role="radio"
          :aria-checked="isSelected(option)"
          :disabled="disabled"
          @click="selectValue(option)"
        >
          {{ optionLabel(option) }}
        </button>
      </div>

      <select
        v-else-if="kind === 'select'"
        :id="fieldId"
        class="ds5t-select"
        :value="selectedOption"
        :disabled="disabled"
        @change="selectValue($event.target.value)"
      >
        <option v-for="option in descriptor.enum_names" :key="option" :value="option">{{ optionLabel(option) }}</option>
      </select>

      <TuningCurve
        v-else-if="kind === 'curve'"
        :model-value="String(modelValue)"
        :label="fieldLabel"
        :disabled="disabled"
        @update:model-value="$emit('update:modelValue', $event)"
      />

      <div v-else-if="kind === 'chips'" class="ds5t-chips" role="group" :aria-labelledby="labelId">
        <button
          v-for="chip in chips"
          :key="chip.name"
          type="button"
          class="ds5t-chip"
          :class="{ selected: chip.checked }"
          :aria-pressed="chip.checked"
          :disabled="disabled"
          @click="toggleChip(chip)"
        >
          {{ optionLabel(chip.name) }}
        </button>
      </div>
    </div>

    <button
      type="button"
      class="ds5t-row-reset"
      :class="{ shown: modified && !disabled }"
      :title="tr('reset_key')"
      :aria-label="tr('reset_key')"
      :tabindex="modified && !disabled ? 0 : -1"
      @click="$emit('reset')"
    >
      <el-icon><RefreshLeft /></el-icon>
    </button>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { RefreshLeft } from '@element-plus/icons-vue'
import TuningCurve from './TuningCurve.vue'
import { useDualSenseText } from '../../composables/useDualSenseText.js'
import { localizedKeyLabel, localizedOptionLabel } from '../../composables/dualsenseLabels.js'
import { formatValue, isModified } from '../../composables/dualsenseTuning.js'

const props = defineProps({
  descriptor: { type: Object, required: true },
  modelValue: { type: [Boolean, Number, String], required: true },
  disabled: { type: Boolean, default: false },
})

const emit = defineEmits(['update:modelValue', 'reset'])
const { tr, locale } = useDualSenseText()

const LOG_STEPS = 1000
const fieldId = computed(() => 'ds5t-key-' + props.descriptor.id)
const labelId = computed(() => 'ds5t-label-' + props.descriptor.id)
const fieldLabel = computed(() => localizedKeyLabel(props.descriptor, locale.value))
const isInteger = computed(() => props.descriptor.type === 'u32' || props.descriptor.type === 'i32')

// A segmented control only stays readable while its labels are short; wide CJK
// characters count in full, Latin letters about half.
const fitsSegmented = computed(() => {
  const names = props.descriptor.enum_names || []
  if (names.length > 4) return false
  const width = names
    .map((name) => localizedOptionLabel(name, locale.value))
    .join('')
    .split('')
    .reduce((sum, character) => sum + (character.charCodeAt(0) > 0x2e80 ? 1 : 0.55), 0)
  return width <= 13
})

const kind = computed(() => {
  switch (props.descriptor.control) {
    case 'slider':
    case 'color_channel':
      return 'slider'
    case 'toggle':
      return 'toggle'
    case 'segmented':
      return fitsSegmented.value ? 'segmented' : 'select'
    case 'curve':
      return 'curve'
    case 'chip_multiselect':
      return 'chips'
    default:
      return 'select'
  }
})
const hasInput = computed(() => ['slider', 'toggle', 'select'].includes(kind.value))

const displayValue = computed(() => formatValue(props.descriptor, props.modelValue))
const modified = computed(() => isModified(props.descriptor, props.modelValue))

const isLog = computed(() => props.descriptor.scale === 'log')
const sliderMin = computed(() => (isLog.value ? 0 : props.descriptor.min))
const sliderMax = computed(() => (isLog.value ? LOG_STEPS : props.descriptor.max))
const sliderStep = computed(() => {
  if (isLog.value) return 1
  const step = Number(props.descriptor.step)
  if (step > 0) return step
  return isInteger.value ? 1 : 0.01
})
const sliderValue = computed(() => {
  if (!isLog.value) return Number(props.modelValue)
  const min = Math.max(Number(props.descriptor.min), 1e-9)
  const max = Number(props.descriptor.max)
  const value = Math.max(min, Math.min(max, Number(props.modelValue)))
  return Math.round((Math.log(value / min) / Math.log(max / min)) * LOG_STEPS)
})
const sliderPercent = computed(() => {
  const span = Number(sliderMax.value) - Number(sliderMin.value)
  if (!(span > 0)) return 0
  return Math.min(100, Math.max(0, ((sliderValue.value - Number(sliderMin.value)) / span) * 100))
})

const chips = computed(() => (props.descriptor.enum_names || []).map((name, index) => ({
  name,
  bit: 1 << index,
  checked: (Number(props.modelValue) & (1 << index)) !== 0,
})))

const selectedOption = computed(() => {
  if (!isInteger.value) return String(props.modelValue)
  return Number(props.modelValue) === 0 && (props.descriptor.enum_names || []).includes('auto')
    ? 'auto'
    : String(props.modelValue)
})

const optionLabel = (option) => localizedOptionLabel(option, locale.value)

const isSelected = (option) => {
  if (isInteger.value) {
    return option === 'auto' ? Number(props.modelValue) === 0 : Number(option) === Number(props.modelValue)
  }
  return option === props.modelValue
}

const selectValue = (option) => {
  if (isInteger.value) emit('update:modelValue', option === 'auto' ? 0 : Number(option))
  else emit('update:modelValue', option)
}

const onSliderInput = (event) => {
  const raw = Number(event.target.value)
  if (!isLog.value) {
    emit('update:modelValue', isInteger.value ? Math.round(raw) : raw)
    return
  }
  const min = Math.max(Number(props.descriptor.min), 1e-9)
  const max = Number(props.descriptor.max)
  let value = min * Math.pow(max / min, raw / LOG_STEPS)
  if (isInteger.value) {
    value = Math.round(value)
  } else {
    const step = Number(props.descriptor.step)
    if (step > 0) value = Math.round(value / step) * step
  }
  emit('update:modelValue', Math.min(max, Math.max(min, value)))
}

const toggleChip = (chip) => {
  const current = Number(props.modelValue)
  emit('update:modelValue', chip.checked ? current & ~chip.bit : current | chip.bit)
}
</script>

<style scoped>
.ds5t-row {
  display: grid;
  grid-template-columns: var(--ds5t-row-cols);
  align-items: center;
  column-gap: var(--ds5t-row-gap);
  min-height: var(--ds5t-row-height);
  padding: var(--ds5t-row-pad) 0;
}

.ds5t-row-label {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  margin: 0;
  color: var(--ds5t-text);
  font-size: 0.875rem;
  font-weight: 500;
  line-height: 1.3;
}

.ds5t-tag {
  border-radius: 999px;
  background: var(--ds5t-danger-soft);
  color: var(--ds5t-danger-text);
  font-size: 0.6875rem;
  font-weight: 600;
  line-height: 1;
  padding: 0.2rem 0.4rem;
  white-space: nowrap;
}

.ds5t-row-control {
  display: flex;
  justify-content: flex-end;
  min-width: 0;
}

.ds5t-row-reset {
  display: grid;
  place-items: center;
  width: 1.5rem;
  height: 1.5rem;
  padding: 0;
  border: 0;
  border-radius: 999px;
  background: transparent;
  color: var(--ds5t-muted);
  font-size: 0.8125rem;
  opacity: 0;
  pointer-events: none;
  cursor: pointer;
  transition: opacity 120ms ease, background-color 120ms ease, color 120ms ease;
}

.ds5t-row-reset.shown {
  opacity: 1;
  pointer-events: auto;
}

.ds5t-row-reset:hover,
.ds5t-row-reset:focus-visible {
  background: var(--ds5t-hover);
  color: var(--ds5t-text);
}

.ds5t-row-reset:focus-visible,
.ds5t-range:focus-visible,
.ds5t-switch:focus-visible,
.ds5t-segment:focus-visible,
.ds5t-chip:focus-visible,
.ds5t-select:focus-visible {
  outline: 2px solid var(--ds5t-accent);
  outline-offset: 2px;
}

/* Slider */
.ds5t-slider {
  display: grid;
  grid-template-columns: minmax(0, 1fr) var(--ds5t-value-w);
  align-items: center;
  column-gap: 0.75rem;
  width: 100%;
}

.ds5t-value {
  color: var(--ds5t-text);
  font-size: 0.8125rem;
  font-variant-numeric: tabular-nums;
  text-align: right;
  white-space: nowrap;
}

.ds5t-range {
  appearance: none;
  width: 100%;
  height: 1.25rem;
  margin: 0;
  background: transparent;
  cursor: pointer;
}

.ds5t-range::-webkit-slider-runnable-track {
  height: 4px;
  border-radius: 999px;
  background: linear-gradient(to right, var(--ds5t-accent) var(--p, 0%), var(--ds5t-track) var(--p, 0%));
}

.ds5t-range::-webkit-slider-thumb {
  appearance: none;
  width: 14px;
  height: 14px;
  margin-top: -5px;
  border: 1.5px solid var(--ds5t-accent);
  border-radius: 50%;
  background: var(--ds5t-thumb);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.22);
}

/* Switch */
.ds5t-switch {
  appearance: none;
  position: relative;
  flex: none;
  width: 2.25rem;
  height: 1.3125rem;
  margin: 0;
  border-radius: 999px;
  background: var(--ds5t-track);
  cursor: pointer;
  transition: background-color 140ms ease;
}

.ds5t-switch::after {
  content: '';
  position: absolute;
  top: 2px;
  left: 2px;
  width: calc(1.3125rem - 4px);
  height: calc(1.3125rem - 4px);
  border-radius: 50%;
  background: var(--ds5t-thumb);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25);
  transition: transform 140ms ease;
}

.ds5t-switch:checked {
  background: var(--ds5t-accent);
}

.ds5t-switch:checked::after {
  transform: translateX(0.9375rem);
}

/* Segmented control */
.ds5t-segmented {
  display: inline-flex;
  max-width: 100%;
  padding: 2px;
  border-radius: var(--ds5t-control-radius);
  background: var(--ds5t-segment-bg);
}

.ds5t-segment {
  min-width: 3.25rem;
  height: 1.75rem;
  padding: 0 0.75rem;
  border: 0;
  border-radius: calc(var(--ds5t-control-radius) - 2px);
  background: transparent;
  color: var(--ds5t-muted);
  font-size: 0.8125rem;
  font-weight: 500;
  white-space: nowrap;
  cursor: pointer;
  transition: background-color 120ms ease, color 120ms ease;
}

.ds5t-segment:hover:not(:disabled):not(.selected) {
  color: var(--ds5t-text);
}

.ds5t-segment.selected {
  background: var(--ds5t-accent);
  color: var(--ds5t-on-accent);
}

/* Select */
.ds5t-select {
  height: 2rem;
  min-width: 9rem;
  max-width: 100%;
  padding: 0 1.75rem 0 0.7rem;
  border: 1px solid var(--ds5t-border);
  border-radius: var(--ds5t-control-radius);
  background-color: var(--ds5t-input-bg);
  color: var(--ds5t-text);
  font-size: 0.8125rem;
}

/* Chips */
.ds5t-chips {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 0.35rem;
}

.ds5t-chip {
  height: 1.75rem;
  padding: 0 0.7rem;
  border: 1px solid var(--ds5t-border);
  border-radius: 999px;
  background: transparent;
  color: var(--ds5t-muted);
  font-size: 0.8125rem;
  cursor: pointer;
}

.ds5t-chip.selected {
  border-color: var(--ds5t-accent);
  background: var(--ds5t-accent);
  color: var(--ds5t-on-accent);
}

/* Unavailable rows read as quiet, not as errors. */
.is-disabled .ds5t-row-label {
  color: var(--ds5t-muted);
}

.is-disabled .ds5t-row-control {
  opacity: 0.55;
}

.is-disabled .ds5t-range,
.is-disabled .ds5t-switch,
.is-disabled .ds5t-segment,
.is-disabled .ds5t-chip,
.is-disabled .ds5t-select {
  cursor: default;
}

/* Blocks that need the full width sit under their label. */
.kind-curve,
.kind-chips {
  grid-template-areas:
    'label reset'
    'control control';
  grid-template-columns: minmax(0, 1fr) 1.5rem;
  row-gap: 0.5rem;
}

.kind-curve .ds5t-row-label,
.kind-chips .ds5t-row-label {
  grid-area: label;
}

.kind-curve .ds5t-row-control,
.kind-chips .ds5t-row-control {
  grid-area: control;
  justify-content: flex-start;
}

.kind-curve .ds5t-row-reset,
.kind-chips .ds5t-row-reset {
  grid-area: reset;
}

/* A narrow card puts the control under the label. */
@container ds5t-card (max-width: 26rem) {
  .ds5t-row {
    grid-template-areas:
      'label reset'
      'control control';
    grid-template-columns: minmax(0, 1fr) 1.5rem;
    row-gap: 0.4rem;
  }

  .ds5t-row-label { grid-area: label; }
  .ds5t-row-control { grid-area: control; justify-content: flex-start; }
  .ds5t-row-reset { grid-area: reset; }
  .kind-toggle {
    grid-template-areas: 'label control reset';
    grid-template-columns: minmax(0, 1fr) auto 1.5rem;
    column-gap: var(--ds5t-row-gap);
  }

  .kind-toggle .ds5t-row-control { justify-content: flex-end; }
}
</style>
