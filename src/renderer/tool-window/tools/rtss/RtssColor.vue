<template>
  <div class="m-rcolor" role="group" :aria-label="label">
    <MSwatches :model-value="hex" :colors="PALETTE" :label="label" @update:model-value="pick" />
    <MField :model-value="draft" :label="hint" :error="valid ? '' : hint" :supporting="valid ? '' : undefined" suffix="#" @update:model-value="typed" />
  </div>
</template>

<script setup>
import { computed, ref, watch } from 'vue'
import { MField, MSwatches } from 'foundation-ui/vue'

// An RTSS colour is six hex digits without #: the swatches are the common ones, the field takes any other; a value that is not six digits is said so and not stored.
const model = defineModel({ type: String, default: 'FFFFFF' })
defineProps({ label: { type: String, required: true }, hint: { type: String, default: 'RRGGBB' } })
const PALETTE = ['FFFFFF', 'FFD700', 'FFA500', 'FF5555', 'FF55FF', '55AAFF', '00FFFF', '00FF00', 'AAAAAA'].map((c) => ({ value: `#${c}`, label: c }))
const draft = ref(model.value)
const valid = computed(() => /^[0-9a-f]{6}$/i.test(draft.value))
const hex = computed(() => `#${String(model.value).toUpperCase()}`)
watch(model, (v) => { if (String(v).toUpperCase() !== draft.value.toUpperCase()) draft.value = v })
const pick = (v) => { draft.value = v.slice(1); model.value = draft.value }
const typed = (v) => { draft.value = String(v).replace(/^#/, '').toUpperCase().slice(0, 6); if (valid.value) model.value = draft.value }
</script>

<style scoped>
.m-rcolor { display: grid; gap: 8px; }
</style>
