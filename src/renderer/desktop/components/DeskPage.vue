<script setup>
import { computed } from 'vue'
import { MHero } from 'foundation-ui/vue'
import { useDeskScale } from '../composables/useDeskScale.js'

// One page of the big-screen window: a hero band (an Expressive shape with the page's icon, its name, one real fact, chips) and the content, at the
// sofa scale. `text` is a fact of the page (a count, the values in force), never a description of what the page is. Without an icon the page has the plain headline.
defineProps({
  title: { type: String, required: true },
  icon: { type: String, default: '' },
  text: { type: String, default: '' },
  tone: { type: String, default: 'primary' },
  wide: { type: Boolean, default: false },
})
const scale = useDeskScale()
const style = computed(() => ({ zoom: scale.value }))
</script>

<template>
  <div class="dsk-page" :class="{ 'dsk-page--wide': wide }" :style="style">
    <MHero v-if="icon" class="dsk-page__hero" :tone="tone" :icon="icon" :title="title" :text="text">
      <template v-if="$slots.chips" #chips><slot name="chips" /></template>
      <slot name="actions" />
    </MHero>
    <header v-else class="dsk-page__head">
      <h1 class="dsk-page__title">{{ title }}</h1>
      <div v-if="$slots.actions" class="dsk-page__actions"><slot name="actions" /></div>
    </header>
    <slot />
  </div>
</template>

<style scoped>
.dsk-page { box-sizing: border-box; max-width: 960px; margin: 0 auto; padding: 8px 0 40px; color: var(--md-sys-color-on-surface); }
.dsk-page--wide { max-width: 1280px; }
.dsk-page__head { display: flex; align-items: center; justify-content: space-between; gap: 16px; margin-bottom: 20px; }
.dsk-page__title { margin: 0; padding: 6px 24px; border-radius: 9999px; background: var(--md-sys-color-surface-container); color: var(--md-sys-color-on-surface); font: var(--m-headline-large); letter-spacing: 0; }
.dsk-page__actions { display: flex; gap: 8px; flex-wrap: wrap; }
/* the hero is the page's one big statement: larger shape, headline-large title, room to breathe */
.dsk-page__hero { margin-bottom: 28px; padding: 30px 40px; gap: 28px; }
.dsk-page__hero :deep(.m-hero__shape) { width: 104px; height: 104px; font-size: 50px; }
.dsk-page__hero :deep(.m-hero__title) { font: var(--m-display-small-emphasized); letter-spacing: -0.01em; }
.dsk-page__hero :deep(.m-hero__text) { font: var(--m-title-large); }
</style>
