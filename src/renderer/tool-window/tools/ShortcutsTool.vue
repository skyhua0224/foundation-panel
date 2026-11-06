<template>
  <!-- the shortcut manual of the Moonlight client: the same shortcuts.md the 帮助与关于 sheet shows, drawn by the shared Markdown component -->
  <div class="m-tool" :class="{ 'is-embedded': embedded }">
    <header v-if="!embedded" class="m-tool__head"><h2>{{ t.shortcutsTool.title }}</h2><MIconButton icon="close" :label="t.toolWindow.close" @click="$emit('close')" /></header>
    <div class="m-tool__body"><MMarkdown :source="shortcutsSource(locale)" /></div>
  </div>
</template>

<script setup>
import { MIconButton, MMarkdown } from 'foundation-ui/vue'
import { shortcutsSource } from './shortcutsSource.js'
import { useI18n } from '../../desktop/i18n/index.js'

const { t, locale } = useI18n()
defineProps({ embedded: { type: Boolean, default: false } })
defineEmits(['close'])
</script>

<style scoped>
.m-tool { display: grid; grid-template-columns: minmax(0, 1fr); gap: 12px; width: 100%; color: var(--md-sys-color-on-surface); }
.m-tool__head { display: flex; align-items: center; justify-content: space-between; gap: 12px; } .m-tool__head h2 { margin: 0; font: var(--m-title-large); }
.m-tool__body { max-height: 85vh; overflow-y: auto; padding: 0 4px 8px; } .is-embedded .m-tool__body { max-height: min(52vh, 520px); }
.m-tool__body :deep(code) { padding: 1px 6px; border-radius: 6px; background: var(--md-sys-color-surface-container-highest); font-family: var(--m-font-mono); font-size: 0.92em; }
</style>
