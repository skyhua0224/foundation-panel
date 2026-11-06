<script setup>
import { computed } from 'vue'
import { useDeskScale } from '../composables/useDeskScale.js'

// One page of the big-screen window: a headline and the content, at the sofa scale. The headline is the page's name and nothing else.
defineProps({
  title: { type: String, required: true },
  wide: { type: Boolean, default: false },
})
const scale = useDeskScale()
const style = computed(() => ({ zoom: scale.value }))
</script>

<template>
  <div class="dsk-page" :class="{ 'dsk-page--wide': wide }" :style="style">
    <header class="dsk-page__head">
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
</style>
