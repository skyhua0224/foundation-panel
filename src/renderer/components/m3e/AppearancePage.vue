<script setup>
import { computed, ref, watch } from 'vue'
import { MAppearance, MPage, MTraySettings } from 'foundation-ui/vue'
import { useI18n } from '../../desktop/i18n/index.js'
import { ROUTES } from '../../composables/useRouter.js'
import { readRtssInstall, rtssInstalled } from '../../composables/rtssInstall.js'

// 个性化 is a section with four pages, like 输入与外设 (the list pane names them, composables/panelNav.js): 主题 (wallpaper, colour, light or dark, icon style, interface), 磨砂与动效 (the frosted
// layer, the animations), 概览 (the tiles of the overview) and 托盘图标 (what the tray icon does, src-tauri/src/tray_prefs.rs). The shared component takes its sections by `part`.
const props = defineProps({ page: { type: String, default: ROUTES.CFG_APPEARANCE } })
const { t, locale } = useI18n()
const PARTS = { [ROUTES.CFG_APPEARANCE]: 'theme', [ROUTES.CFG_FROST]: 'frost', [ROUTES.CFG_OVERVIEW]: 'home' }
const part = computed(() => PARTS[props.page] || '')
const isTray = computed(() => props.page === ROUTES.CFG_TRAY)
const names = computed(() => t.value.m3e.appearance)
const title = computed(() => (isTray.value ? names.value.tray : props.page === ROUTES.CFG_FROST ? names.value.frost : props.page === ROUTES.CFG_OVERVIEW ? names.value.overview : names.value.theme))

const prefs = ref({})
const invokeTray = async (name, args) => {
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke(name, args)
}
// the tray record and the install check of RTSS are read where the tray page is, not for the pages that do not use them
watch(isTray, async (on) => {
  if (!on) return
  try { prefs.value = await invokeTray('tray_prefs_get') } catch { /* not the app window: the defaults are shown */ }
  // the tile list of the quick panel offers RTSS only where it is installed
  if (rtssInstalled.value === null) await readRtssInstall((name) => invokeTray(name))
}, { immediate: true })
// the tiles and sliders of the quick panel go by their own command: the rest of the record is saved without them
const saveLayout = async (record) => {
  prefs.value = { ...prefs.value, tiles: record.tiles, sliders: record.sliders }
  try { prefs.value = await invokeTray('tray_layout_set', { tiles: record.tiles, sliders: record.sliders }) } catch { /* not saved: the next start has the old layout */ }
}
const save = async (next) => {
  prefs.value = next // the choice shows at once; the host answers with the record it kept
  try { prefs.value = await invokeTray('tray_prefs_set', { prefs: next }) } catch { /* not saved: the next start has the old choice */ }
}
</script>

<template>
  <MPage :title="title">
    <MTraySettings v-if="isTray" :model-value="prefs" :locale="String(locale)" :rtss="rtssInstalled === true" @update:model-value="save" @layout="saveLayout" />
    <MAppearance v-else :locale="String(locale)" :part="part" />
  </MPage>
</template>
