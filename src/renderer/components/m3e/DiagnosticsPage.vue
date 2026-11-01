<template>
  <MPage :title="t.m3e.system.diagnostics" :lead="t.m3e.system.diagnosticsNote" :eyebrow="t.m3e.system.title" back :back-label="t.m3e.back" @back="emit('back')">
    <DeviceDiagnosticsPanel @open-controller-meta="emit('open-controller-meta')" @open-stylus-input-probe="emit('open-stylus-input-probe')" />
    <MSection :title="t.sidebar.toolbox">
      <MGroup>
        <MItem v-for="tool in utilityTools" :key="tool.id" action :chevron="!tool.external" :icon="ICONS[tool.id] || 'build'" :title="tool.label" :text="tool.description" @click="tool.action()">
          <template v-if="tool.external" #trail><MIcon name="open_in_new" /></template>
        </MItem>
      </MGroup>
    </MSection>
  </MPage>
</template>

<script setup>
import { computed } from 'vue'
import { MPage, MSection, MGroup, MItem, MIcon } from 'foundation-ui/vue'
import DeviceDiagnosticsPanel from '../controllersHub/DeviceDiagnosticsPanel.vue'
import { useI18n } from '../../desktop/i18n/index.js'

const props = defineProps({ tools: { type: Array, default: () => [] } })
const emit = defineEmits(['back', 'open-controller-meta', 'open-stylus-input-probe'])
const { t } = useI18n()
// The controller test and the stylus probe are the two cards of the device tools above.
const utilityTools = computed(() => props.tools.filter((tool) => !['gamepad-test', 'stylus-input-probe'].includes(tool.id)))
const ICONS = { 'stream-timer': 'timer', 'latency-test': 'speed' }
</script>
