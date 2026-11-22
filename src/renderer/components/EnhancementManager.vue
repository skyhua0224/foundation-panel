<template>
  <MSection :title="kind === 'nr' ? 'DLSS NR' : 'RTX HDR'">
    <MStatusPill :icon="kind === 'nr' ? 'wand_stars' : 'light_mode'" :label="componentState" :tone="status.enabled && statusKnown && status.installed ? 'ok' : ''" :action="false" />
    <MFooter>{{ kind === 'nr' ? controlsText.nrIntro : controlsText.hdrIntro }}</MFooter>

    <MBanner v-if="statusKnown && !status.host_supported" tone="warn" icon="warning">{{ text.bridgeMissingNotice }}</MBanner>
    <MBanner v-else-if="statusKnown && !status.adapter_present" tone="warn" icon="warning">{{ text.adapterMissingNotice }}</MBanner>
    <MBanner v-else-if="statusKnown && !status.vc_runtime_present" tone="warn" icon="warning">
      {{ text.vcRuntimeMissingNotice }}
      <template #actions><MButton variant="text" size="sm" icon="download" @click="openVcRuntimeDownload">{{ text.vcRuntimeDownload }}</MButton></template>
    </MBanner>
    <MBanner v-else-if="statusKnown && !status.runtime_present" tone="info" icon="info">{{ text.runtimeMissingNotice }}</MBanner>
    <MBanner v-if="status.maintenance" tone="warn" icon="build">
      {{ maintenanceMessage }}
      <template v-if="status.state !== 'maintenance_other'" #actions><MButton variant="text" size="sm" :disabled="controlsBusy" :loading="operation === 'recovering'" @click="recover">{{ status.state === 'pending_removal' ? text.retryRemoval : text.recover }}</MButton></template>
    </MBanner>
    <MBanner v-if="status.in_use" tone="warn" icon="warning">{{ text.inUseNotice }}</MBanner>
    <MBanner v-if="operationError" tone="error" icon="error"><b>{{ text.technicalDetails }}</b><br>{{ operationError }}<template #actions><MButton variant="text" size="sm" @click="operationError = ''">×</MButton></template></MBanner>

    <MGroup v-if="status.installed">
      <MSwitchItem
        :model-value="status.enabled"
        icon="power_settings_new"
        :title="kind === 'nr' ? controlsText.liveTitle : controlsText.allowUse + 'RTX HDR'"
        :text="kind === 'nr' ? controlsText.nrPermissionHint : controlsText.hdrPermissionHint"
        :disabled="!statusKnown || !status.installed || (!status.enabled && status.state === 'repair_required') || status.maintenance || controlsBusy"
        @update:model-value="setEnabled"
      />
      <MItem v-if="kind === 'nr' && status.enabled" action icon="tune" :title="controlsText.appDefaults" @click="openApplicationSettings" />
    </MGroup>

    <div v-if="statusKnown && !status.installed" class="m-actions-row">
      <MButton icon="download" :loading="operation === 'install'" :disabled="controlsBusy || !status.host_supported || !status.adapter_present || status.in_use || status.maintenance" @click="install">{{ controlsText.installSetup }}</MButton>
      <MButton variant="text" icon="help" @click="showAcquisition">{{ controlsText.setupHelp }}</MButton>
    </div>

    <MGroup>
      <MItem action icon="folder_special" :title="text.acquisitionTitle" @click="showAcquisition" />
      <MItem v-if="statusKnown" action icon="upload" :title="status.installed ? controlsText.replaceFile : controlsText.installSetup" :disabled="!status.host_supported || !status.adapter_present || status.in_use || status.maintenance || controlsBusy || operation === 'install'" @click="install" />
      <MItem action icon="refresh" :title="text.refresh" :disabled="refreshing || controlsBusy" @click="refresh()" />
      <MItem v-for="item in healthRows" :key="item.label" :icon="/ok|good|ready|success/i.test(String(item.tone)) ? 'check_circle' : 'warning'" :title="item.label" :value="item.state" />
      <MItem v-if="status.installed" action icon="delete" :title="text.uninstall" :disabled="status.in_use || status.maintenance || controlsBusy || operation === 'uninstall'" @click="uninstall" />
    </MGroup>
    <MFooter>{{ text.intro }}</MFooter>
  </MSection>
</template>

<script setup>
import { enhancementControlsText } from '../composables/enhancementControls.js'
import { MSection, MGroup, MItem, MSwitchItem, MStatusPill, MBanner, MFooter, MButton } from 'foundation-ui/vue'
import { computed } from 'vue'
import { dlssNr, rtxHdr } from '../tauri-adapter.js'
import { dlssNrText } from '../composables/dlssNrMessages.js'
import { useI18n } from '../desktop/i18n/index.js'
import { useRtxHdrI18n } from '../composables/rtxHdrI18n.js'
import { useEnhancementManager } from '../composables/useEnhancementManager.js'
import { noteDlssNrStatus } from '../composables/useDlssNrReady.js'

const props = defineProps({ kind: { type: String, default: 'hdr' } })
const { locale } = useI18n()
const controlsText = computed(() => enhancementControlsText(locale.value))
const hdrText = useRtxHdrI18n()
const text = computed(() => ({
  ...(props.kind === 'nr' ? dlssNrText(locale.value) : hdrText.value),
  saveSuccess: controlsText.value.permissionSaved,
  permissionSaveFailed: controlsText.value.permissionSaveFailed
}))

const {
  status,
  statusKnown,
  refreshing,
  operation,
  operationError,
  controlsBusy,
  stateLabel,
  healthRows,
  refresh,
  install,
  uninstall,
  setEnabled,
  showAcquisition,
  openVcRuntimeDownload,
  openApplicationSettings,
  recover
} = useEnhancementManager({
  api: props.kind === 'nr' ? dlssNr : rtxHdr,
  messages: text,
  runtimeName: props.kind === 'nr' ? 'nvngx_dlssnr.dll' : 'nvngx_truehdr.dll',
  // the DLSS NR live controls elsewhere follow what this page reads (install, repair, uninstall)
  onStatus: props.kind === 'nr' ? noteDlssNrStatus : undefined
})
const maintenanceMessage = computed(() => {
  if (status.value.state === 'pending_removal') return text.value.pendingRemovalNotice
  if (status.value.state === 'maintenance_other') return text.value.otherMaintenanceNotice
  return text.value.maintenanceNotice
})
const componentState = computed(() => {
  if (
    !statusKnown.value ||
    ['repair_required', 'unavailable', 'degraded'].includes(
      status.value.state
    ) ||
    status.value.maintenance
  )
    return stateLabel.value
  if (!status.value.installed) return controlsText.value.needsInstall
  if (
    !status.value.adapter_present ||
    !status.value.vc_runtime_present ||
    !status.value.runtime_present
  )
    return controlsText.value.needsSetup
  return status.value.enabled
    ? controlsText.value.ready
    : controlsText.value.installedDisabled
})
</script>
