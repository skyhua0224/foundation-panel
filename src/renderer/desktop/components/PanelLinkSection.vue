<script setup>
import { useI18n } from '../i18n/index.js'
import PanelLink from './PanelLink.vue'

// A heading (default: more in the Panel) and the tiles "在 Panel 中打开" of one page: `links` are keys of PANEL_LINKS (panelLinks.js).
defineProps({ links: { type: Array, required: true }, heading: { type: String, default: '' } })
const { t } = useI18n()
</script>

<template>
  <section class="dsk-links">
    <h2 class="dsk-links__title">{{ heading || t.panel.more }}</h2>
    <div class="dsk-links__grid">
      <PanelLink v-for="link in links" :key="link" :link="link" />
    </div>
  </section>
</template>

<style scoped>
.dsk-links { margin-top: 36px; }
.dsk-links__title { margin: 0 0 16px 4px; font: var(--m-headline-small-emphasized); color: var(--md-sys-color-on-surface); letter-spacing: 0; }
.dsk-links__grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 16px; }
@media (max-width: 1300px) { .dsk-links__grid { grid-template-columns: minmax(0, 1fr); } }
</style>
