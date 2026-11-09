<template>
  <MItem
    :icon="icon"
    :title="name"
    :text="description"
    :disabled="disabled"
    :stack="stack"
  >
    <template #title><slot name="name">{{ name }}</slot></template>
    <template v-if="$slots.description" #default><slot name="description" /></template>
    <template v-if="stack" #control><slot /></template>
    <template v-else #trail><slot /></template>
  </MItem>
</template>

<script setup>
import { MItem } from 'foundation-ui/vue'

// One row of a settings section: the name, one line of what it does, the control at the end (or below it when `stack`). The same MItem row as the Web UI, the Panel and the phone.
defineProps({
  name: { type: String, default: '' },
  description: { type: String, default: '' },
  icon: { type: String, default: '' },
  disabled: { type: Boolean, default: false },
  stack: { type: Boolean, default: false },
})
</script>
