<template>
  <SettingsCard :title="title" :icon="icon">
    <SettingsRow
      v-for="item in items"
      :key="item.key"
      class="dsk-toggle"
      :class="{ 'dsk-toggle--on': !item.disabled }"
      :name="item.name"
      :description="item.description"
      :disabled="Boolean(item.disabled)"
      @click="onRow($event, item)"
    >
      <SettingsSwitch
        :model-value="Boolean(values[item.key])"
        :disabled="Boolean(item.disabled)"
        :label="item.name"
        @update:model-value="emit('update-value', item.key, $event)"
      />
    </SettingsRow>
  </SettingsCard>
</template>

<script setup>
import SettingsCard from './SettingsCard.vue'
import SettingsRow from './SettingsRow.vue'
import SettingsSwitch from './SettingsSwitch.vue'

const props = defineProps({
  title: { type: String, required: true },
  icon: { type: String, required: true },
  items: { type: Array, required: true },
  values: { type: Object, required: true },
})

const emit = defineEmits(['update-value'])

// The whole row is the target: a press anywhere on it but the switch itself flips the switch.
function onRow(event, item) {
  if (item.disabled || event.target?.closest?.('.m-switch')) return
  emit('update-value', item.key, !props.values[item.key])
}
</script>

<style scoped>
.dsk-toggle--on { cursor: pointer; }
.dsk-toggle--on:hover { background: color-mix(in srgb, var(--md-sys-color-on-surface) 6%, var(--m-card)); }
</style>
