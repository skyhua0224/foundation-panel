<template>
  <SettingsCard :title="title">
    <SettingsToggleRow
      v-for="item in items"
      :key="item.key"
      :name="item.name"
      :description="item.description"
      :disabled="Boolean(item.disabled)"
      :model-value="Boolean(values[item.key])"
      @update:model-value="emit('update-value', item.key, $event)"
    />
  </SettingsCard>
</template>

<script setup>
import SettingsCard from './SettingsCard.vue'
import SettingsToggleRow from './SettingsToggleRow.vue'

defineProps({
  title: { type: String, required: true },
  items: { type: Array, required: true },
  values: { type: Object, required: true },
})

const emit = defineEmits(['update-value'])
</script>
