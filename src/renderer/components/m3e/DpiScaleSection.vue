<template>
  <MSection id="m-desktop-scale" :title="d.title">
    <MBanner v-if="problem" tone="error" icon="error">{{ problem }}</MBanner>
    <MGroup>
      <MSliderItem v-model="dpi" icon="format_size" :title="d.scale" :min="100" :max="300" :step="25" :decimals="0" unit="%" size="m" icon-start="text_decrease" icon-end="text_increase" :readout-text="loading ? d.loading : ''" :disabled="applying || loading" />
      <MItem stack :title="d.presets"><template #control><MChipSet v-model="dpi" :label="d.presets" :options="DPI_PRESETS.map((v) => ({ value: v, label: `${v}%` }))" :disabled="applying || loading" /></template></MItem>
    </MGroup>
    <MFooter v-if="note">{{ note }}</MFooter>
    <div class="m-dpi__actions"><MButton icon="check" :loading="applying" :disabled="applying || loading || dpi === systemDpi" @click="apply">{{ applying ? d.applying : d.apply }}</MButton></div>
  </MSection>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { MSection, MGroup, MItem, MSliderItem, MChipSet, MButton, MFooter, MBanner, useNotify } from 'foundation-ui/vue'
import { useI18n } from '../../desktop/i18n/index.js'

// 桌面缩放: the old DPI 调节 tool (get_current_dpi / set_desktop_dpi) on the virtual display page; it scales the desktop the client sees.
const DPI_PRESETS = [100, 125, 150, 175, 200, 225, 250, 300]
const { t } = useI18n()
const d = computed(() => t.value.m3e.dpi)
const dpi = ref(100), systemDpi = ref(null), loading = ref(true), applying = ref(false), note = ref(''), problem = ref('')
// a failure stays on the page with the host's own words (SetDpi.exe missing, could not run it, not Windows), until the next call succeeds
async function read() {
  try { systemDpi.value = await invoke('get_current_dpi') } catch (e) { systemDpi.value = null; problem.value = d.value.readFailed.replace('{error}', String(e)) }
  return systemDpi.value
}
onMounted(async () => { const v = await read(); if (v) dpi.value = v; loading.value = false })
async function apply() {
  applying.value = true
  note.value = ''
  problem.value = ''
  const wanted = dpi.value
  try {
    await invoke('set_desktop_dpi', { dpi: wanted })
    useNotify().snack(d.value.applied.replace('{value}', wanted))
    // Windows can keep another value (a per-monitor limit); say what it shows after a second, as the tool did
    setTimeout(async () => { const now = await read(); if (now !== null && now !== wanted) note.value = d.value.partial.replace('{value}', now) }, 1000)
  } catch (e) {
    problem.value = d.value.failed.replace('{error}', String(e))
  } finally { applying.value = false }
}
</script>

<style>
.m-dpi__actions { display: flex; gap: 8px; padding: 8px 0 4px; }
</style>
