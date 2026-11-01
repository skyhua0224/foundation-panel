<template>
  <MPage :title="t.m3e.system.components" :lead="t.deviceHub.components.managementHint" :eyebrow="t.m3e.system.title">
    <UsbTransportPanel :transport="transport" maintenance />
    <InputDriversPanel />
  </MPage>
</template>

<script setup>
import { MPage } from 'foundation-ui/vue'
import InputDriversPanel from '../controllersHub/InputDriversPanel.vue'
import UsbTransportPanel from '../controllersHub/UsbTransportPanel.vue'
import { useUsbTransport } from '../../composables/useUsbTransport.js'
import { useI18n } from '../../desktop/i18n/index.js'

const emit = defineEmits(['back'])
const { t } = useI18n()
const transport = useUsbTransport()
</script>
