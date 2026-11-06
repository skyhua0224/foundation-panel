<template>
  <MItem
    class="dsk-row"
    :class="{ 'dsk-row--tool': nameClass }"
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

// One row of a settings section: the name, one line of what it does, the control at the end (or below it when `stack`).
defineProps({
  name: { type: String, default: '' },
  description: { type: String, default: '' },
  nameClass: { type: [String, Array, Object], default: null },
  controlClass: { type: [String, Array, Object], default: null },
  disabled: { type: Boolean, default: false },
  stack: { type: Boolean, default: false },
})
</script>

<style scoped>
.dsk-row { min-height: 72px; }
.dsk-row :deep(.m-item__title) { font: var(--m-title-medium); }
.dsk-row--tool :deep(.m-item__title) { display: inline-flex; align-items: center; gap: 10px; }
</style>
