<template>
  <MSection :title="title" :description="lead">
    <MGroup role="list"><slot /></MGroup>
    <div v-if="$slots.extra" class="dsk-card__extra"><slot name="extra" /></div>
  </MSection>
</template>

<script setup>
import { MGroup, MSection } from 'foundation-ui/vue'

// A settings section: the label, the rows as one connected group, like the settings of the Web UI, the Panel and the phone. `lead` is one line that says what the whole section does.
defineProps({
  title: { type: String, required: true },
  lead: { type: String, default: '' },
})
</script>

<style scoped>
.dsk-card__extra { display: flex; flex-direction: column; gap: 8px; margin-top: 8px; }
</style>
