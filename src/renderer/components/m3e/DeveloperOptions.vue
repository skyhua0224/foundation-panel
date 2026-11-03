<template>
  <MGroup>
    <MSwitchItem :model-value="Boolean(settings.devMode)" :title="zh ? '调试输出' : 'Debug output'" :info="zh ? '打开面板的调试输出（例如开发者工具）；日常使用保持关闭。' : 'Turns on the debug output of the panel (such as the developer tools); leave off for daily use.'" data-panel-option="devMode" @update:model-value="set($event)" />
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
