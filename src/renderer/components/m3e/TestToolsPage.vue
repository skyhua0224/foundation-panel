<template>
  <MPage :title="T.title">
    <MTestTools :locale="locale" />
  </MPage>
</template>

<script setup>
import { computed } from 'vue'
import { MPage, MTestTools, testToolTexts } from 'foundation-ui/vue'
import { useI18n } from '../../desktop/i18n/index.js'

// 测试工具 (toolbox): 画面测试图, 帧率与卡顿, 音画同步, 声道, 键盘鼠标触控, 防休眠; the shared page of foundation-ui.
const { locale } = useI18n()
const T = computed(() => testToolTexts(locale.value))
</script>
