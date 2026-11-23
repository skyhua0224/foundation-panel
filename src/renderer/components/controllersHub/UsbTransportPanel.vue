<template>
  <MSection :title="t.deviceHub.components.transportTitle" :description="t.deviceHub.components.transportHint">
    <MGroup>
      <MItem icon="cable" :tone="statusClass === 'state-error' ? 'error' : statusClass === 'state-ready' ? 'success' : ''" :title="statusTitle" :text="status.version ? 'v' + status.version : ''">
        <template #trail>
          <MIconButton icon="refresh" :label="t.deviceHub.refresh" :disabled="transportBusy || statusLoading" @click="refreshStatus({ force: true })" />
        </template>
      </MItem>
    </MGroup>
    <MGroup v-if="fixOffered">
      <MItem icon="build" :title="t.deviceHub.usb.fixTitle">
        <template #trail><MButton size="sm" variant="primary" :loading="fixing" :disabled="transportBusy" @click="fixTransport()">{{ t.deviceHub.usb.fixAction }}</MButton></template>
      </MItem>
    </MGroup>
    <MNotice v-if="statusProbeFailed || (status.detail && status.installed)" tone="error">{{ friendlyError(status.detail) }}</MNotice>
    <MGroup v-if="statusLoaded && !statusProbeFailed && !status.externally_managed && status.supported && !status.ready && !needsCleanup">
      <MItem action icon="download" :title="t.deviceHub.usb.installTransport" :disabled="transportBusy || installing" @click="installTransport" />
    </MGroup>
    <MNotice v-if="statusLoaded && !statusProbeFailed && status.supported && staleResidual" tone="warn">
      {{ t.deviceHub.usb.staleHint }}
      <template #action><MButton size="sm" variant="primary" :loading="cleaning" :disabled="transportBusy" @click="removeStale">{{ t.deviceHub.usb.removeStale }}</MButton></template>
    </MNotice>
    <MNotice v-else-if="statusLoaded && !statusProbeFailed && status.supported && needsCleanup" tone="warn">
      {{ t.deviceHub.usb.residualHint }}
      <template #action><MButton size="sm" variant="danger" :loading="cleaning" :disabled="transportBusy" @click="cleanupResidual">{{ t.deviceHub.usb.cleanupResidual }}</MButton></template>
    </MNotice>
    <MNotice v-if="status.externally_managed" :tone="status.ready ? 'info' : 'warn'">
      {{ status.ready ? t.deviceHub.usb.externalReuse : canReplaceLegacy ? t.deviceHub.usb.externalReplaceable : t.deviceHub.usb.externalDecision }}
      <template v-if="canReplaceLegacy" #action><MButton size="sm" variant="danger" :loading="replacing" :disabled="transportBusy || attachedDevices.length > 0" @click="replaceLegacy">{{ t.deviceHub.usb.replaceLegacy }}</MButton></template>
    </MNotice>
    <MNotice v-if="status.cleanup_pending_restart" tone="warn">{{ t.deviceHub.usb.restartPending }}</MNotice>
    <MNotice v-else-if="status.install_pending_restart" tone="warn">{{ t.deviceHub.usb.installRestartPending }}</MNotice>
    <MNotice v-else-if="status.reboot_recommended" tone="warn">{{ t.deviceHub.usb.rebootRequired }}</MNotice>
    <MGroup v-if="status.detail || status.last_install_error || status.last_cleanup_error || fixRaw">
      <MItem action icon="description" :title="t.deviceHub.usb.diagnosticDetails" @click="detailOpen = !detailOpen"><template #trail><MIcon :name="detailOpen ? 'keyboard_arrow_up' : 'keyboard_arrow_down'" /></template></MItem>
      <MItem v-if="detailOpen" plain :title="''"><pre class="m-pre">{{ [fixRaw, status.detail, status.last_install_error ? t.deviceHub.usb.lastInstallError + ': ' + status.last_install_error : '', status.last_cleanup_error ? t.deviceHub.usb.lastCleanupError + ': ' + status.last_cleanup_error : ''].filter(Boolean).join('\n') }}</pre></MItem>
    </MGroup>
    <MGroup v-if="statusLoaded && status.supported && (status.detail || status.last_install_error || status.last_cleanup_error || status.vhci_residual || (status.installed && !status.ready))">
      <MItem action icon="download" :title="t.deviceHub.usb.exportDiagnostics" :disabled="exporting" @click="exportDiagnostics" />
    </MGroup>
    <MGroup v-if="maintenance && !status.externally_managed && statusLoaded && !statusProbeFailed && status.installed && !needsCleanup">
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
const { status, statusLoading, statusLoaded, statusProbeFailed, installing, cleaning, replacing, exporting, fixing, fixOffered, fixRaw, attachedDevices, needsCleanup, staleResidual, canReplaceLegacy, transportBusy, statusClass, statusTitle, friendlyError, refreshStatus, installTransport, cleanupResidual, removeStale, exportDiagnostics, replaceLegacy, fixTransport } = transport
</script>
