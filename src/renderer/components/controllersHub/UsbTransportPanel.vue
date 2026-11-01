<template>
  <MSection :title="t.deviceHub.components.transportTitle" :description="t.deviceHub.components.transportHint">
    <MGroup>
      <MItem icon="cable" :tone="statusClass === 'state-error' ? 'error' : statusClass === 'state-ready' ? 'success' : ''" :title="statusTitle" :text="status.version ? 'v' + status.version : ''">
        <template #trail>
          <MIconButton icon="refresh" :label="t.deviceHub.refresh" :disabled="transportBusy || statusLoading" @click="refreshStatus({ force: true })" />
        </template>
      </MItem>
    </MGroup>
    <MNotice v-if="statusProbeFailed || (status.detail && status.installed)" tone="error">{{ friendlyError(status.detail) }}</MNotice>
    <MGroup v-if="statusLoaded && !statusProbeFailed && status.supported && !status.ready && !needsCleanup">
      <MItem action icon="download" :title="t.deviceHub.usb.installTransport" :disabled="transportBusy || installing" @click="installTransport" />
    </MGroup>
    <MNotice v-if="statusLoaded && !statusProbeFailed && status.supported && needsCleanup" tone="warn">
      {{ t.deviceHub.usb.residualHint }}
      <template #action><MButton size="sm" variant="danger" :loading="cleaning" :disabled="transportBusy" @click="cleanupResidual">{{ t.deviceHub.usb.cleanupResidual }}</MButton></template>
    </MNotice>
    <MNotice v-if="status.reboot_recommended" tone="warn">{{ t.deviceHub.usb.rebootRequired }}</MNotice>
    <MGroup v-if="status.detail">
      <MItem action icon="description" :title="t.deviceHub.usb.diagnosticDetails" @click="detailOpen = !detailOpen"><template #trail><MIcon :name="detailOpen ? 'keyboard_arrow_up' : 'keyboard_arrow_down'" /></template></MItem>
      <MItem v-if="detailOpen" plain :title="''"><pre class="m-pre">{{ status.detail }}</pre></MItem>
    </MGroup>
    <MGroup v-if="maintenance && statusLoaded && !statusProbeFailed && status.installed && !needsCleanup">
      <MItem icon="build" :title="t.deviceHub.components.maintenance">
        <template #trail><MButton size="sm" variant="danger" :loading="cleaning" :disabled="transportBusy || attachedDevices.length > 0" @click="cleanupResidual">{{ t.deviceHub.components.uninstallTransport }}</MButton></template>
      </MItem>
    </MGroup>
  </MSection>
</template>

<script setup>
import { ref } from 'vue'
import { MSection, MGroup, MItem, MNotice, MButton, MIcon, MIconButton } from 'foundation-ui/vue'
import { useI18n } from '../../desktop/i18n/index.js'
const { t } = useI18n()
const detailOpen = ref(false)
const { transport } = defineProps({ transport: { type: Object, required: true }, maintenance: Boolean })
const { status, statusLoading, statusLoaded, statusProbeFailed, installing, cleaning, attachedDevices, needsCleanup, transportBusy, statusClass, statusTitle, friendlyError, refreshStatus, installTransport, cleanupResidual } = transport
</script>
