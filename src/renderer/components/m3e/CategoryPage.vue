<script setup>
import { computed } from 'vue'
import { MPage, MSection, MGroup, MItem, MChip, MHero } from 'foundation-ui/vue'
import { microphoneOverviewState } from '../../composables/deviceHubStatus.js'
import { useI18n } from '../../desktop/i18n/index.js'

const props = defineProps({
  category: { type: String, required: true },
  runtime: { type: Object, required: true },
})
const emit = defineEmits(['open'])
const { t } = useI18n()
const m = computed(() => t.value.m3e)

const dsState = computed(() => props.runtime.ds?.state || '')
const dsReady = computed(() => ['ready', 'active'].includes(dsState.value))
const micStates = computed(() => t.value.deviceHub.microphone.statusLabels)

// Each entry: id, icon, title, supporting text, optional live status chip.
const entries = computed(() => {
  const c = m.value
  const lists = {
    stream: [
      ['clients', 'qr_code_2', ...c.stream.clients], ['apps', 'apps', ...c.stream.apps],
      ['settings', 'tune', ...c.stream.settings], ['downloads', 'download', ...c.stream.downloads],
    ].map(([id, icon, title, text]) => ({ id, icon, title, text })),
    input: [
      { id: 'gamepad', icon: 'sports_esports', title: c.input.gamepad, text: c.input.gamepadNote, status: dsReady.value ? { tone: 'ok', text: t.value.dualSense.status.states[dsState.value] || t.value.deviceHub.available } : null },
      { id: 'microphone', icon: 'mic', title: c.input.microphone, text: c.input.micNote, status: { tone: 'neutral', text: micStates.value[micState.value] || '' } },
      { id: 'usb', icon: 'usb', title: c.input.usb, text: c.input.usbNote, status: props.runtime.usb?.ready ? { tone: 'ok', text: t.value.deviceHub.available } : null },
    ],
    display: [
      { id: 'vdd', icon: 'desktop_windows', title: c.display.vdd, text: c.display.vddNote },
      { id: 'quality', icon: 'wand_stars', title: c.display.quality, text: c.display.qualityNote },
    ],
    system: [
      { id: 'components', icon: 'extension', title: c.system.components, text: c.system.componentsNote },
      { id: 'diagnostics', icon: 'stethoscope', title: c.system.diagnostics, text: c.system.diagnosticsNote },
      { id: 'assistant', icon: 'smart_toy', title: c.system.assistant, text: c.system.assistantNote },
    ],
  }
  return lists[props.category] || []
})
const micState = computed(() => microphoneOverviewState(props.runtime.mic || {}))
const readyCount = computed(() => [dsReady.value, !!props.runtime.usb?.ready, props.runtime.mic?.component_available].filter(Boolean).length)
const head = computed(() => ({
  stream: { title: m.value.stream.title, lead: m.value.stream.lead },
  input: { title: m.value.input.title, lead: m.value.input.lead },
  display: { title: m.value.display.title, lead: m.value.display.lead },
  system: { title: m.value.system.title, lead: m.value.system.lead },
}[props.category]))
const lead = { gamepad: 'primary', microphone: 'tertiary', usb: 'success' }
</script>

<template>
  <MPage :title="head.title" :lead="head.lead">
    <MHero v-if="category === 'input'" :tone="readyCount === 3 ? 'success' : 'tertiary'" :icon="readyCount === 3 ? 'check_circle' : 'info'" :title="m.input.summary.replace('{count}', readyCount).replace('{total}', 3)" />
    <MSection>
      <MGroup>
        <MItem v-for="entry in entries" :key="entry.id" action chevron :icon="entry.icon" :title="entry.title" :text="entry.text" :tone="lead[entry.id] || ''" @click="emit('open', entry.id)">
          <template v-if="entry.status" #trail><MChip :tone="entry.status.tone" :icon="entry.status.tone === 'ok' ? 'check_circle' : ''">{{ entry.status.text }}</MChip></template>
        </MItem>
      </MGroup>
    </MSection>
  </MPage>
</template>
