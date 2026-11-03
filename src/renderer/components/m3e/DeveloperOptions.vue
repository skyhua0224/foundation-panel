<template>
  <MGroup>
    <MSwitchItem :model-value="Boolean(settings.devMode)" :title="st.devMode" :info="st.devModeDesc" data-panel-option="devMode" @update:model-value="set($event)" />
  </MGroup>
</template>

<script setup>
import { MGroup, MSwitchItem } from 'foundation-ui/vue'
import { useI18n } from '../../desktop/i18n/index.js'
import { desktopSettings, saveDesktopSettings } from '../../desktop/composables/useDesktopSettings.js'

// What the panel adds to the 开发者 entry of 关于: the debug output of the window. It is only reachable once developer mode is on (seven taps on the version).
const { t } = useI18n()
const st = t.value ? { get devMode() { return t.value.settings.devMode }, get devModeDesc() { return t.value.settings.devModeDesc } } : {}
const settings = desktopSettings
const set = (value) => saveDesktopSettings({ ...settings.value, devMode: value }).catch(() => {})
defineExpose({ off: () => set(false) })
</script>
