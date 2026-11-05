<script setup>
import { computed } from 'vue'
import { MAppearance, MPage } from 'foundation-ui/vue'
import { labelOf } from 'foundation-ui/settings/index.js'
import { useI18n } from '../../desktop/i18n/index.js'

// 个性化 among the stream settings: the shared appearance component (wallpaper, colour source, light or dark, dim, blur, icon style)
const { locale } = useI18n()
const title = computed(() => labelOf(locale.value, 'shell.appearance'))
</script>

<template>
  <MPage :title="title">
    <MAppearance :locale="String(locale)" />
  </MPage>
</template>
