<template>
  <MSwitch :model-value="modelValue" :disabled="disabled" :label="label" @update:model-value="handleChange" />
</template>

<script setup>
import { MSwitch } from 'foundation-ui/vue'

defineProps({
  modelValue: { type: Boolean, required: true },
  disabled: { type: Boolean, default: false },
  label: { type: String, default: '' },
})

const emit = defineEmits(['update:modelValue', 'change'])

function handleChange(nextValue) {
  emit('update:modelValue', nextValue)
  emit('change', nextValue)
}
</script>
