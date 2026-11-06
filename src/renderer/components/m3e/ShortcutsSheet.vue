<template>
  <MSheet :model-value="modelValue" :title="t.toolbar.shortcutGuide" side="end" :close-label="t.m3e.home.done" @update:model-value="(v) => emit('update:modelValue', v)">
    <div class="m-shortcuts"><MMarkdown :source="shortcutsSource(locale)" /></div>
  </MSheet>
</template>

<script setup>
import { MSheet, MMarkdown } from 'foundation-ui/vue'
import { useI18n } from '../../desktop/i18n/index.js'
import { shortcutsSource } from '../../tool-window/tools/shortcutsSource.js'

// 快捷键手册: the Moonlight client shortcuts (the same shortcuts.md the old tool window showed), on 帮助与关于 and from the toolbar menu.
defineProps({ modelValue: { type: Boolean, default: false } })
const emit = defineEmits(['update:modelValue'])
const { t, locale } = useI18n()
</script>

<style>
.m-shortcuts { padding: 0 4px 16px; }
.m-shortcuts code { padding: 1px 6px; border-radius: 6px; background: var(--md-sys-color-surface-container-highest); font-family: var(--m-font-mono); font-size: 0.92em; }
</style>
