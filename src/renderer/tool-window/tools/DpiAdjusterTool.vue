<template>
  <!-- the old DPI tool lives on as the 桌面缩放 section of the virtual display page (same get_current_dpi / set_desktop_dpi, same 1 s read-back); this window shows that one section -->
  <div class="m-tool" :class="{ 'is-embedded': embedded }">
    <header v-if="!embedded" class="m-tool__head"><h2>{{ t.dpiTool.title }}</h2><MIconButton icon="close" :label="t.toolWindow.close" @click="$emit('close')" /></header>
    <DpiScaleSection />
  </div>
</template>

<script setup>
import { MIconButton } from 'foundation-ui/vue'
import { useI18n } from '../../desktop/i18n/index.js'
import DpiScaleSection from '../../components/m3e/DpiScaleSection.vue'

const { t } = useI18n()
defineProps({ embedded: { type: Boolean, default: false } })
defineEmits(['close'])
</script>

<style scoped>
.m-tool { display: grid; grid-template-columns: minmax(0, 1fr); gap: 16px; width: 100%; padding: 4px 0; color: var(--md-sys-color-on-surface); }
.m-tool__head { display: flex; align-items: center; justify-content: space-between; gap: 12px; } .m-tool__head h2 { margin: 0; font: var(--m-title-large); }
</style>
