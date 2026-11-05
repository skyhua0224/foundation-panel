<template>
  <MPage :title="T.title">
    <MLatencyClock :locale="locale" can-stream @fullscreen-stream="(o) => openLatencyClockOnStream(o)" @open-external="openUrl" />
  </MPage>
</template>

<script setup>
import { computed } from 'vue'
import { MPage, MLatencyClock, latencyClockTexts } from 'foundation-ui/vue'
import { useI18n } from '../../desktop/i18n/index.js'
import { useTools } from '../../composables/useTools.js'

// 延迟测试时钟 (toolbox): the clock in the panel, and full screen in a window of its own on the display the stream captures (useTools.openLatencyClockOnStream).
const { locale } = useI18n()
const T = computed(() => latencyClockTexts(locale.value))
const { openLatencyClockOnStream, openUrl } = useTools()
</script>
