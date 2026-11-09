<script setup>
import { computed, onMounted, ref } from 'vue'
import { MAppearance, MPage, MSection, MTraySettings } from 'foundation-ui/vue'
import { labelOf } from 'foundation-ui/settings/index.js'
import { useI18n } from '../../desktop/i18n/index.js'

// 个性化 among the stream settings: the shared appearance component (wallpaper, colour source, light or dark, dim, blur, icon style), and what the tray icon does (src-tauri/src/tray_prefs.rs)
const { locale } = useI18n()
const title = computed(() => labelOf(locale.value, 'shell.appearance'))
const trayTitle = computed(() => ({ zh: '托盘图标', ja: 'トレイアイコン' }[String(locale.value)] || 'Tray icon'))

const prefs = ref({})
const invokeTray = async (name, args) => {
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke(name, args)
}
onMounted(async () => {
  try { prefs.value = await invokeTray('tray_prefs_get') } catch { /* not the app window: the defaults are shown */ }
})
const save = async (next) => {
  prefs.value = next // the choice shows at once; the host answers with the record it kept
  try { prefs.value = await invokeTray('tray_prefs_set', { prefs: next }) } catch { /* not saved: the next start has the old choice */ }
}
</script>

<template>
  <MPage :title="title">
    <MAppearance :locale="String(locale)" />
    <MSection :title="trayTitle">
      <MTraySettings :model-value="prefs" :locale="String(locale)" @update:model-value="save" />
    </MSection>
  </MPage>
</template>
