<template>
  <MPage :title="t.m3e.input.usb" :lead="t.deviceHub.intro" :eyebrow="t.m3e.input.title">
    <UsbPassthroughPanel />
  </MPage>
</template>

<script setup>
import { MPage } from 'foundation-ui/vue'
import UsbPassthroughPanel from '../controllersHub/UsbPassthroughPanel.vue'
import { useI18n } from '../../desktop/i18n/index.js'

const emit = defineEmits(['back', 'manage-components'])
const { t } = useI18n()
</script>
