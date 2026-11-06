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
/* A section is one tonal tile: a shaped icon, the name, the rows connected under it. */
.dsk-card { position: relative; overflow: hidden; isolation: isolate; margin: 0; padding: 22px 20px 20px; border-radius: var(--md-sys-shape-corner-extra-extra-large); background: var(--md-sys-color-surface-container); display: flex; flex-direction: column; gap: 14px; }
.dsk-card::after { content: ''; position: absolute; z-index: -1; right: -56px; top: -64px; width: 220px; height: 220px; background: var(--md-sys-color-primary); opacity: .07; clip-path: var(--m-shape-cookie); animation: m-turn 52s linear infinite; }
.dsk-card__head { display: flex; align-items: center; gap: 16px; padding: 0 4px; }
.dsk-card__icon { position: relative; flex: none; width: 60px; height: 60px; display: grid; place-items: center; font-size: 30px; color: var(--md-sys-color-on-primary); }
.dsk-card__icon::before { content: ''; position: absolute; inset: 0; background: var(--md-sys-color-primary); clip-path: var(--m-shape-burst); animation: m-turn 40s linear infinite; }
.dsk-card__icon :deep(.m-icon) { position: relative; }
.dsk-card__title { margin: 0; flex: 1; min-width: 0; font: var(--m-headline-small-emphasized); color: var(--md-sys-color-on-surface); letter-spacing: 0; }
.dsk-card__actions { display: flex; gap: 8px; }
.dsk-card__lead { margin: 0 4px; font: var(--m-body-large); color: var(--md-sys-color-on-surface-variant); }
.dsk-card__extra { display: flex; flex-direction: column; gap: 8px; margin-top: 8px; }
</style>
