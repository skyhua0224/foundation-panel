<template>
  <MChoiceItem
    v-if="kind === 'select'"
    :model-value="selectedOption"
    :title="fieldLabel"
    :options="segOptions"
    :disabled="disabled"
    :cancel-label="tr('cancel')"
    :search-label="tr('search')"
    :data-ds5-key="descriptor.id"
    @update:model-value="selectValue"
  >
    <template v-if="descriptor.expert || reset" #trail><MChip v-if="descriptor.expert" tone="warn">{{ tr('expert') }}</MChip><MIconButton v-if="reset" icon="restart_alt" :label="tr('reset_key')" @click.stop="$emit('reset')" /></template>
  </MChoiceItem>

  <MItem v-else :title="fieldLabel" :disabled="disabled" :stack="stacked" class="ds5-field" :class="{ 'is-modified': modified }" :data-ds5-key="descriptor.id" :data-tip="disabled ? tr('pending.reason') : undefined">
    <template #trail>
      <MChip v-if="descriptor.expert" tone="warn">{{ tr('expert') }}</MChip>
      <span v-if="kind === 'slider'" class="ds5-readout">{{ displayValue }}</span>
      <MIconButton v-if="reset" icon="restart_alt" :label="tr('reset_key')" @click="$emit('reset')" />
      <MSwitch v-if="kind === 'toggle'" :model-value="modelValue === true" :label="fieldLabel" :disabled="disabled" @update:model-value="$emit('update:modelValue', $event)" />
    </template>
    <template v-if="stacked" #control>
      <MSlider
        v-if="kind === 'slider'"
        :model-value="Number(modelValue)"
        :min="Number(descriptor.min)"
        :max="Number(descriptor.max)"
        :step="sliderStep"
        :decimals="decimals"
        :unit="unitSuffix"
        :log="descriptor.scale === 'log'"
        :centered="centered"
        :center="centered ? 0 : null"
        size="m"
        :icon-start="icon"
        :disabled="disabled"
        :label="fieldLabel"
        @update:model-value="onSlider"
        @dblclick="$emit('reset')"
      />
      <MSeg v-else-if="kind === 'segmented'" block :label="fieldLabel" :model-value="selectedOption" :options="segOptions" :disabled="disabled" @update:model-value="selectValue" />
      <TuningCurve v-else-if="kind === 'curve'" :model-value="String(modelValue)" :label="fieldLabel" :disabled="disabled" @update:model-value="$emit('update:modelValue', $event)" />
      <MSeg v-else-if="kind === 'chips'" multiple block :label="fieldLabel" :model-value="chipValues" :options="segOptions" :disabled="disabled" @update:model-value="setChips" />
    </template>
  </MItem>
</template>

<script setup>
import { MChip, MChoiceItem, MIconButton, MItem, MSeg, MSlider, MSwitch } from 'foundation-ui/vue'
import { computed } from 'vue'
import TuningCurve from './TuningCurve.vue'
import { sliderIcon } from './tuningIcons.js'
import { useDualSenseText } from '../../composables/useDualSenseText.js'
import { localizedKeyLabel, localizedOptionLabel } from '../../composables/dualsenseLabels.js'
import { isModified } from '../../composables/dualsenseTuning.js'

// One tuning key as an Expressive row. The same rows, in the same order of preference, as the Web page: switch, connected
// button group (short names), searchable picker (long lists), slider with inset icon and value indicator (log and centred
// variants), curve, chips as a toggle group.
const props = defineProps({
  descriptor: { type: Object, required: true },
  modelValue: { type: [Boolean, Number, String], required: true },
  disabled: { type: Boolean, default: false },
})
const emit = defineEmits(['update:modelValue', 'reset'])
const { tr, locale } = useDualSenseText()

const fieldLabel = computed(() => localizedKeyLabel(props.descriptor, locale.value))
const isInteger = computed(() => props.descriptor.type === 'u32' || props.descriptor.type === 'i32')
const numeric = computed(() => ['f32', 'u32', 'i32'].includes(props.descriptor.type))
const optionLabel = (option) => localizedOptionLabel(option, locale.value)

// A connected button group only stays readable while its labels are short; wide CJK characters count in full, Latin about half.
const fitsSegmented = computed(() => {
  const names = props.descriptor.enum_names || []
  if (names.length > 4) return false
  const width = names.map((name) => optionLabel(name)).join('').split('').reduce((sum, character) => sum + (character.charCodeAt(0) > 0x2e80 ? 1 : 0.55), 0)
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
const stacked = computed(() => ['slider', 'segmented', 'curve', 'chips'].includes(kind.value))

const decimals = computed(() => {
  const step = Number(props.descriptor.step)
  if (isInteger.value || !Number.isFinite(step) || step >= 1 || step <= 0) return 0
  return Math.min(6, Math.max(0, Math.ceil(-Math.log10(step))))
})
const unitSuffix = computed(() => (props.descriptor.unit === 'deg' ? '°' : props.descriptor.unit ? ' ' + props.descriptor.unit : ''))
const displayValue = computed(() => {
  const number = Number(props.modelValue)
  return (numeric.value && Number.isFinite(number) ? number.toFixed(decimals.value) : String(props.modelValue)) + unitSuffix.value
})
const sliderStep = computed(() => {
  const step = Number(props.descriptor.step)
  if (step > 0) return step
  return isInteger.value ? 1 : 0.01
})
// A value around zero (an offset) is drawn from the middle outwards
const centered = computed(() => Number(props.descriptor.min) < 0 && Number(props.descriptor.max) > 0)
const icon = computed(() => sliderIcon(props.descriptor.key))
const onSlider = (value) => emit('update:modelValue', isInteger.value ? Math.round(Number(value)) : Number(value))

const modified = computed(() => isModified(props.descriptor, props.modelValue))
const reset = computed(() => modified.value && !props.disabled)

const segOptions = computed(() => (props.descriptor.enum_names || []).map((name) => ({ value: name, label: optionLabel(name) })))
const selectedOption = computed(() => {
  if (!isInteger.value) return String(props.modelValue)
  return Number(props.modelValue) === 0 && (props.descriptor.enum_names || []).includes('auto') ? 'auto' : String(props.modelValue)
})
const selectValue = (option) => emit('update:modelValue', isInteger.value ? (option === 'auto' ? 0 : Number(option)) : option)

// A multi-select value is a bit mask: one bit per option name
const chipValues = computed(() => (props.descriptor.enum_names || []).filter((name, index) => (Number(props.modelValue) & (1 << index)) !== 0))
const setChips = (names) => {
  const all = props.descriptor.enum_names || []
  emit('update:modelValue', names.reduce((mask, name) => mask | (1 << all.indexOf(name)), 0))
}
</script>

<style>
.ds5-readout { font: var(--m-label-large); font-variant-numeric: tabular-nums; color: var(--md-sys-color-primary); text-align: right; }
.ds5-field.is-modified .m-item__title::after { content: ''; display: inline-block; width: 6px; height: 6px; margin-inline-start: 8px; border-radius: 50%; background: var(--md-sys-color-primary); vertical-align: middle; }
</style>
