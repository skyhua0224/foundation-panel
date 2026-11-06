<template>
  <MSection class="dsk-card">
    <div class="dsk-card__head">
      <span class="dsk-card__icon"><MIcon :name="icon" fill /></span>
      <h2 class="dsk-card__title">{{ title }}</h2>
      <div v-if="$slots.actions" class="dsk-card__actions"><slot name="actions" /></div>
    </div>
    <p v-if="lead" class="dsk-card__lead">{{ lead }}</p>
    <MGroup role="list" class="dsk-card__rows"><slot /></MGroup>
    <div v-if="$slots.extra" class="dsk-card__extra"><slot name="extra" /></div>
  </MSection>
</template>

<script setup>
import { MGroup, MIcon, MSection } from 'foundation-ui/vue'

// A settings section: a tonal icon, the name, the rows as one connected group. `lead` is one line that says what the whole section does.
defineProps({
  title: { type: String, required: true },
  icon: { type: String, required: true },
  lead: { type: String, default: '' },
})
</script>

<style scoped>
.dsk-card { margin: 0; }
.dsk-card__head { display: flex; align-items: center; gap: 12px; padding: 0 4px 4px; }
.dsk-card__icon { flex: none; width: 40px; height: 40px; border-radius: 14px; display: grid; place-items: center; font-size: 24px; background: var(--md-sys-color-secondary-container); color: var(--md-sys-color-on-secondary-container); }
.dsk-card__title { margin: 0; flex: 1; min-width: 0; font: var(--m-title-large); color: var(--md-sys-color-on-surface); letter-spacing: 0; }
.dsk-card__actions { display: flex; gap: 8px; }
.dsk-card__lead { margin: 0 4px 4px; font: var(--m-body-medium); color: var(--md-sys-color-on-surface-variant); }
.dsk-card__extra { display: flex; flex-direction: column; gap: 8px; margin-top: 8px; }
</style>
