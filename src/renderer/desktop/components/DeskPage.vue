<script setup>
import { computed } from 'vue'
import { MPage } from 'foundation-ui/vue'
import { useDeskScale } from '../composables/useDeskScale.js'

// One page of the big-screen window: the same page as everywhere else in the app (MPage: the title row, then sections of rows), drawn at the
// sofa scale (useDeskScale: 1.25 times, more with the theme editor's font size). No hero band, no shapes: the phone, the Web UI and the Panel
// open their pages the same way. `actions` are the page's buttons beside the title.
defineProps({
  title: { type: String, required: true },
  wide: { type: Boolean, default: false },
})
const scale = useDeskScale()
const style = computed(() => ({ zoom: scale.value }))
</script>

<template>
  <div class="dsk-page" :style="style">
    <MPage :title="title" :wide="wide">
      <template v-if="$slots.actions" #actions><slot name="actions" /></template>
      <slot />
    </MPage>
  </div>
</template>

<style scoped>
.dsk-page { box-sizing: border-box; color: var(--md-sys-color-on-surface); }
/* the window's own surface is the page, so the rows are the tonal step above it (in the Panel the page is that step's parent: rows stand out from it the same way) */
.dsk-page :deep(.m-page) { --m-card: var(--md-sys-color-surface-container); }
/* the window's own column scrolls and has its own inset: the page adds none of its own */
.dsk-page :deep(.m-page) { --m-gutter: 0px; padding-bottom: 40px; }
</style>

<style>
/* the panel's global heading colour (dark slate) would sit on the rows in dark mode: the page's title takes the colour of the page */
.dsk-page h1 { color: inherit; }
</style>
