<template>
  <div class="m-usb">
    <UsbForwardingSettings :transport-ready="status.ready && !statusProbeFailed" />
    <UsbTransportPanel :transport="transport" />
    <MSection>
      <MGroup>
        <MItem action icon="folder_open" :title="t.deviceHub.usb.manualTitle" @click="manualOpen = !manualOpen"><template #trail><MInfo :text="t.deviceHub.usb.exporterHint" /><MIcon :name="manualOpen ? 'keyboard_arrow_up' : 'keyboard_arrow_down'" /></template></MItem>
      </MGroup>
      <form v-if="manualOpen" class="m-usb__discover" @submit.prevent="discover">
        <md-outlined-text-field class="m-usb__host" data-redact="address" :label="t.deviceHub.usb.hostPlaceholder" :value="remote" autocomplete="off" :disabled="!status.ready || discovering" @input="remote = $event.target.value"><span slot="container" class="m-field__fill" aria-hidden="true" /></md-outlined-text-field>
        <md-outlined-text-field class="m-port-field" type="number" min="1024" max="65535" :label="t.deviceHub.usb.tcpPortLabel" :value="tcpPort" :disabled="!status.ready || discovering" @input="tcpPort = Number($event.target.value)"><span slot="container" class="m-field__fill" aria-hidden="true" /></md-outlined-text-field>
        <MButton type="submit" :loading="discovering" :disabled="!canDiscover">{{ t.deviceHub.usb.discover }}</MButton>
      </form>
      <MGroup v-if="manualOpen && discoveryDone" aria-live="polite">
        <MItem v-if="remoteDevices.length === 0" plain :title="t.deviceHub.usb.noRemoteDevices" />
        <MItem v-for="device in remoteDevices" :key="device.bus_id" data-redact="serial" icon="usb" :title="device.description || t.deviceHub.usb.unknownDevice" :text="t.deviceHub.usb.busId + ' · ' + device.bus_id + (device.details?.length ? ' · ' + device.details[0] : '')">
          <template #trail><MButton size="sm" variant="tonal" :loading="attachingBusId === device.bus_id" :disabled="operationBusy || !status.ready" @click="attachDevice(device)">{{ t.deviceHub.usb.attach }}</MButton></template>
        </MItem>
      </MGroup>
    </MSection>
    <MSection :title="t.deviceHub.usb.attachedTitle" :description="t.deviceHub.usb.attachedHint">
      <MEmpty v-if="statusLoaded && attachedDevices.length === 0" icon="usb" :title="t.deviceHub.usb.noAttachedDevices" />
      <MGroup v-else>
        <MItem v-for="device in attachedDevices" :key="device.port" data-redact="address" icon="usb" tone="success" :title="device.description || t.deviceHub.usb.unknownDevice" :text="device.remote_host + ':' + device.remote_port + ' · ' + device.remote_bus_id + ' · ' + device.speed + (device.serial ? ' · ' + device.serial : '')">
          <template #trail>
            <MChip>{{ String(device.port).padStart(2, '0') }}</MChip>
            <MButton size="sm" variant="danger" :loading="detachingPort === device.port" :disabled="operationBusy" @click="detachDevice(device)">{{ t.deviceHub.usb.detach }}</MButton>
          </template>
        </MItem>
      </MGroup>
    </MSection>
  </div>
</template>

<script setup>
import { MSection, MGroup, MItem, MNotice, MButton, MIcon, MInfo, MChip, MEmpty } from 'foundation-ui/vue'
import { computed, ref, watch } from 'vue'
import { ElMessage, ElMessageBox } from '../../composables/notifyMessage.js'
import { usbip } from '../../tauri-adapter.js'
import { useI18n } from '../../desktop/i18n/index.js'
import { useUsbTransport } from '../../composables/useUsbTransport.js'
import { usbipStatusChannel } from '../../composables/deviceStatusStore.js'
import UsbForwardingSettings from './UsbForwardingSettings.vue'
import UsbTransportPanel from './UsbTransportPanel.vue'

const { t } = useI18n()
const manualOpen = ref(false)
const transport = useUsbTransport()
const { status, statusLoaded, statusProbeFailed, attachedDevices, refreshStatus, friendlyError, transportBusy } = transport
const discovering = ref(false)
const discoveryDone = ref(false)
const attachingBusId = ref('')
const detachingPort = ref(0)
const remote = ref('')
const tcpPort = ref(3240)
const remoteDevices = ref([])
const operationBusy = computed(() => transportBusy.value || Boolean(attachingBusId.value || detachingPort.value))
const validTcpPort = computed(() => Number.isInteger(tcpPort.value) && tcpPort.value >= 1024 && tcpPort.value <= 65535)
const canDiscover = computed(() => Boolean(
  status.ready && remote.value.trim() && validTcpPort.value && !discovering.value && !operationBusy.value,
))

async function discover() {
  if (!canDiscover.value) return
  discovering.value = true
  discoveryDone.value = false
  const result = await usbip.listRemote(remote.value.trim(), tcpPort.value)
  discovering.value = false
  discoveryDone.value = true
  if (!result?.success) {
    remoteDevices.value = []
    await refreshStatus({ force: true, afterCurrent: true })
    return ElMessage.error(friendlyError(result?.message))
  }
  remoteDevices.value = result.data || []
}

async function attachDevice(device) {
  if (!status.ready || operationBusy.value) return
  try {
    await ElMessageBox.confirm(
      t.value.deviceHub.usb.attachConfirm
        .replace('{device}', device.description || device.bus_id)
        .replace('{host}', remote.value.trim()),
      t.value.deviceHub.usb.attach,
      { confirmButtonText: t.value.deviceHub.usb.attach, cancelButtonText: t.value.deviceHub.usb.cancel, type: 'warning' },
    )
  } catch { return }
  if (!status.ready || operationBusy.value) return
  attachingBusId.value = device.bus_id
  const result = await usbip.attach(remote.value.trim(), device.bus_id, tcpPort.value)
  attachingBusId.value = ''
  // 变更落定后先失效通道：随后的强制刷新不得并入变更前发出的在途探测。
  usbipStatusChannel.invalidate()
  if (!result?.success) {
    await refreshStatus({ force: true, afterCurrent: true })
    return ElMessage.error(friendlyError(result?.message))
  }
  await refreshStatus({ force: true, afterCurrent: true })
  ElMessage.success(t.value.deviceHub.usb.attachSuccess)
}

async function detachDevice(device) {
  try {
    await ElMessageBox.confirm(
      t.value.deviceHub.usb.detachConfirm.replace('{device}', device.description || device.remote_bus_id),
      t.value.deviceHub.usb.detach,
      { confirmButtonText: t.value.deviceHub.usb.detach, cancelButtonText: t.value.deviceHub.usb.cancel, type: 'warning', danger: true },
    )
  } catch { return }
  detachingPort.value = device.port
  const result = await usbip.detach(device.port)
  detachingPort.value = 0
  // 变更落定后先失效通道：随后的强制刷新不得并入变更前发出的在途探测。
  usbipStatusChannel.invalidate()
  if (!result?.success) {
    await refreshStatus({ force: true, afterCurrent: true })
    return ElMessage.error(friendlyError(result?.message))
  }
  await refreshStatus({ force: true, afterCurrent: true })
  ElMessage.success(t.value.deviceHub.usb.detachSuccess)
}

watch([remote, tcpPort], () => {
  discoveryDone.value = false
  remoteDevices.value = []
})
</script>
