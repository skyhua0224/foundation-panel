<script setup>
import { MGroup, MSection } from 'foundation-ui/vue'
import { useI18n } from '../i18n/index.js'
import PanelLink from './PanelLink.vue'

// A heading (default: more in the Panel) and the rows "在 Panel 中打开" of one page: `links` are keys of PANEL_LINKS (panelLinks.js).
defineProps({ links: { type: Array, required: true }, heading: { type: String, default: '' } })
const { t } = useI18n()
</script>

<template>
  <MSection :title="heading || t.panel.more">
    <MGroup>
      <PanelLink v-for="link in links" :key="link" :link="link" />
    </MGroup>
  </MSection>
</template>
