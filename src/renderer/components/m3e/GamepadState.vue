<script setup>
import { computed } from 'vue'
import { MPage, MSection, MGroup, MItem, MStatusPill, MIcon } from 'foundation-ui/vue'
import { useI18n } from '../../desktop/i18n/index.js'

// Supporting pane of the gamepad page (windows 1200 px and wider): the live state that the host reports, next to the tuning.
const props = defineProps({ runtime: { type: Object, required: true } })
const { t } = useI18n()
const ds = computed(() => props.runtime.ds || {})
const states = computed(() => t.value.dualSense.status.states)
const ready = computed(() => ['ready', 'active'].includes(ds.value.state))
const yes = (v) => (v ? t.value.deviceHub.available : t.value.deviceHub.unavailable)
const rows = computed(() => [
  ['dns', t.value.m3e.state.host, ds.value.host ? yes(ds.value.host.reachable) : ''],
  ['sports_esports', t.value.m3e.state.pads, ds.value.host?.active_controllers != null ? (ds.value.host.active_controllers > 0 ? t.value.m3e.state.connected.replace('{n}', ds.value.host.active_controllers) : t.value.m3e.state.none) : ''],
  ['usb', t.value.m3e.state.usb, ds.value.transport ? (ds.value.transport.version || yes(ds.value.transport.installed)) : ''],
].filter((r) => r[2]))
</script>

<template>
  <MPage title="DualSense">
    <MStatusPill :icon="ready ? 'check_circle' : 'info'" :label="states[ds.state] || '–'" :tone="ready ? 'ok' : ''" :action="false" />
    <MSection v-if="rows.length">
      <MGroup>
        <MItem v-for="[icon, title, value] in rows" :key="title" :icon="icon" :title="title" :value="value" />
      </MGroup>
    </MSection>
  </MPage>
</template>
