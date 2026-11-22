<template>
  <MComponentStatus :status="status">
    <section id="component-maintenance" ref="maintenance" class="panel-maint">
      <UsbTransportPanel :transport="transport" maintenance />
      <InputDriversPanel />
      <VbCableComponentPanel />
    </section>
  </MComponentStatus>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { MComponentStatus, registeredComponentStatus } from 'foundation-ui/vue'
import InputDriversPanel from '../controllersHub/InputDriversPanel.vue'
import VbCableComponentPanel from '../controllersHub/VbCableComponentPanel.vue'
import UsbTransportPanel from '../controllersHub/UsbTransportPanel.vue'
import { useUsbTransport } from '../../composables/useUsbTransport.js'
import { loadVmouseSupport, vmouseSupported } from '../../composables/useVmouse.js'

// 组件状态 and 组件 in one destination: the status cards on top (the controller is the one the shell registered; the home cards read the same
// one), the install, update, repair and cleanup panels of the components below. The update and repair buttons of a card scroll to them.
const controller = registeredComponentStatus()
// foundation-ui owns the complete component list, so filter the unsupported
// optional card at this page boundary instead of showing an "unknown" Windows
// 10 component. The same capability is used by the maintenance panel.
const status = { ...controller, rows: computed(() => controller.rows.value.filter((row) => row.id !== 'mouse' || vmouseSupported.value)) }
const transport = useUsbTransport()
const maintenance = ref(null)
const scrollTo = () => maintenance.value?.scrollIntoView({ behavior: 'smooth', block: 'start' })
onMounted(() => { void loadVmouseSupport(); controller.hooks = { ...(controller.hooks || {}), scrollToMaintenance: scrollTo } })
onBeforeUnmount(() => { if (controller.hooks) controller.hooks.scrollToMaintenance = null })
</script>

<style>
.panel-maint { margin-top: 12px; display: flex; flex-direction: column; gap: 4px; }
</style>
