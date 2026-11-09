<template>
  <MItem
    class="dsk-toggle"
    :class="{ 'dsk-toggle--on': !disabled }"
    :icon="icon"
    :title="name"
    :text="description"
    :disabled="disabled"
    :stack="Boolean($slots.default)"
    @click="onRow"
  >
    <template #trail><MSwitch :model-value="modelValue" :disabled="disabled" :label="name" @update:model-value="(on) => emit('update:modelValue', on)" /></template>
    <template v-if="$slots.default" #control><slot /></template>
  </MItem>
</template>

<script setup>
import { MItem, MSwitch } from 'foundation-ui/vue'

// One row of a big-screen settings group: a name, one line of what it does, the switch at the end. The whole row is the target (a pad presses the
// row's switch, a remote or a finger the row); what the row shows below the name when it is on (a choice of modes) is the default slot and
// keeps its own presses. The same row everywhere on the big screen, the same MItem as the Web UI, the Panel and the phone.
const props = defineProps({
  modelValue: { type: Boolean, required: true },
  name: { type: String, required: true },
  description: { type: String, default: '' },
  icon: { type: String, default: '' },
  disabled: { type: Boolean, default: false },
})
const emit = defineEmits(['update:modelValue'])

function onRow(event) {
  if (props.disabled || event.target?.closest?.('.m-switch, .m-item__control')) return
  emit('update:modelValue', !props.modelValue)
}
</script>

<style scoped>
.dsk-toggle--on { cursor: pointer; }
</style>
