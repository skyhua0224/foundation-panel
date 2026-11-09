<script setup>
import { computed } from 'vue'
import { MIcon, MItem, useNotify } from 'foundation-ui/vue'
import { useI18n } from '../i18n/index.js'
import { PANEL_LINKS, openInPanel } from '../panelLinks.js'

// "在 Panel 中打开": one row for one page of the main panel, the same MItem row as every list of the app (the icon, the page's name, what is on it,
// the arrow out). A pad presses it like any row.
const props = defineProps({ link: { type: String, required: true } })
const { t } = useI18n()
const meta = PANEL_LINKS[props.link]
const text = computed(() => t.value.panel.links[props.link])

async function open() {
  if (!(await openInPanel(meta.page))) useNotify().snack(t.value.panel.openFailed, { timeout: 4000 })
}
</script>

<template>
  <MItem
    action
    :icon="meta.icon"
    :title="text.title"
    :text="text.items"
    :data-focus-key="'panel-' + link"
    :aria-label="`${t.panel.open}: ${text.title}`"
    @click="open"
  >
    <template #trail><MIcon name="open_in_new" /></template>
  </MItem>
</template>
