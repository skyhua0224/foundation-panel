<template>
  <MComponentStatus :status="status">
    <section id="component-maintenance" ref="maintenance" class="panel-maint">
      <UsbTransportPanel :transport="transport" maintenance />
      <InputDriversPanel />
    </section>
  </MComponentStatus>
</template>

<script setup>
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { MComponentStatus, registeredComponentStatus } from 'foundation-ui/vue'
import InputDriversPanel from '../controllersHub/InputDriversPanel.vue'
import UsbTransportPanel from '../controllersHub/UsbTransportPanel.vue'
import { useUsbTransport } from '../../composables/useUsbTransport.js'

// 组件状态 and 组件 in one destination: the status cards on top (the controller is the one the shell registered; the home cards read the same
// one), the install, update, repair and cleanup panels of the components below. The update and repair buttons of a card scroll to them.
const status = registeredComponentStatus()
const transport = useUsbTransport()
const maintenance = ref(null)
const scrollTo = () => maintenance.value?.scrollIntoView({ behavior: 'smooth', block: 'start' })
onMounted(() => { status.hooks = { ...(status.hooks || {}), scrollToMaintenance: scrollTo } })
onBeforeUnmount(() => { if (status.hooks) status.hooks.scrollToMaintenance = null })
</script>

<style>
.panel-maint { margin-top: 12px; display: flex; flex-direction: column; gap: 4px; }
</style>
