<template>
  <MSeg
    class="dsk-select"
    :model-value="modelValue"
    :options="options"
    :label="label"
    :block="block"
    :disabled="disabled"
    @update:model-value="handleChange"
  />
</template>

<script setup>
import { MSeg } from 'foundation-ui/vue'

// A choice among a few short values: connected buttons, every value in sight (nothing hides behind a drop-down a pad cannot open).
defineProps({
  modelValue: { type: [String, Number], required: true },
  options: { type: Array, required: true },
  label: { type: String, default: '' },
  block: { type: Boolean, default: true },
  disabled: { type: Boolean, default: false },
})

const emit = defineEmits(['update:modelValue', 'change'])

function handleChange(nextValue) {
  emit('update:modelValue', nextValue)
  emit('change', nextValue)
}
</script>
