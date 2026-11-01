<template>
  <div
    class="m-item m-tuning"
    :class="['kind-' + kind, { 'm-item--stack': stacked, 'm-item--plain': true, 'is-disabled': disabled, 'is-modified': modified }]"
    :title="disabled ? tr('pending.reason') : null"
  >
    <div class="m-item__body">
      <label :id="labelId" :for="hasInput ? fieldId : null" class="m-item__title">
        <span>{{ fieldLabel }}</span>
        <span v-if="descriptor.expert" class="m-chip m-chip--warn m-tuning__tag">{{ tr('expert') }}</span>
      </label>
    </div>
    <div class="m-item__trail">
      <output v-if="kind === 'slider'" class="m-slider-value" :for="fieldId">{{ displayValue }}</output>
      <md-switch
        v-else-if="kind === 'toggle'"
        :id="fieldId"
        :selected="modelValue === true"
        :disabled="disabled"
        :aria-labelledby="labelId"
        @change="$emit('update:modelValue', $event.target.selected)"
      />
      <md-outlined-select
        v-else-if="kind === 'select'"
        :id="fieldId"
        class="m-tuning__select"
        :value="selectedOption"
        :disabled="disabled"
        :aria-labelledby="labelId"
        @change="selectValue($event.target.value)"
      >
        <md-select-option v-for="option in descriptor.enum_names" :key="option" :value="option" :selected="option === selectedOption">
          <div slot="headline">{{ optionLabel(option) }}</div>
        </md-select-option>
      </md-outlined-select>
      <button
        type="button"
        class="m-icon-btn m-tuning__reset"
        :class="{ shown: modified && !disabled }"
        :title="tr('reset_key')"
        :aria-label="tr('reset_key')"
        :tabindex="modified && !disabled ? 0 : -1"
        @click="$emit('reset')"
      ><MIcon name="restart_alt" /></button>
    </div>
    <div v-if="stacked" class="m-item__control">
      <md-slider
        v-if="kind === 'slider'"
        :id="fieldId"
        :min="sliderMin"
        :max="sliderMax"
        :step="sliderStep"
        :value="sliderValue"
        :disabled="disabled"
        :aria-labelledby="labelId"
        @input="onSliderInput"
        @dblclick="$emit('reset')"
      />
      <MSeg
        v-else-if="kind === 'segmented'"
        block
        :label="fieldLabel"
        :model-value="selectedOption"
        :options="descriptor.enum_names.map((name) => ({ value: name, label: optionLabel(name) }))"
        :disabled="disabled"
        @update:model-value="selectValue"
      />
      <TuningCurve
        v-else-if="kind === 'curve'"
        :model-value="String(modelValue)"
        :label="fieldLabel"
        :disabled="disabled"
        @update:model-value="$emit('update:modelValue', $event)"
      />
      <div v-else-if="kind === 'chips'" class="m-tuning__chips" role="group" :aria-labelledby="labelId">
        <button
          v-for="chip in chips"
          :key="chip.name"
          type="button"
          class="m-tuning__chip"
          :aria-pressed="chip.checked"
          :disabled="disabled"
          @click="toggleChip(chip)"
        >{{ optionLabel(chip.name) }}</button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { MIcon, MSeg } from 'foundation-ui/vue'
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

const stacked = computed(() => ['slider', 'segmented', 'curve', 'chips'].includes(kind.value))
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


