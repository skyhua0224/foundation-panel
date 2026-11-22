<template>
  <MSection :title="text.title">
    <MGroup>
      <MItem icon="cable" :title="text.device" :text="deviceText" wrap>
        <template #trail>
          <MButton size="sm" variant="text" icon="refresh" :loading="phase === 'check'" :disabled="busy" @click="check">{{ text.check }}</MButton>
        </template>
      </MItem>
      <MItem icon="inventory_2" :title="text.package" :text="archiveText" wrap>
        <template #trail>
          <MButton size="sm" variant="tonal" icon="download" :loading="phase === 'download'" :disabled="busy || archiveReady" @click="download">{{ text.download }}</MButton>
        </template>
      </MItem>
      <MItem icon="settings" :title="text.manage" :text="text.separateHint" wrap>
        <template #trail>
          <MButton size="sm" variant="tonal" :loading="phase === 'install'" :disabled="!canInstall" @click="changeDriver('install')">{{ props.installLabel || text.install }}</MButton>
          <MButton v-if="status?.state === 'disabled'" size="sm" variant="tonal" :loading="phase === 'enable'" :disabled="!canEnable" @click="changeDriver('enable')">{{ text.repair }}</MButton>
          <MButton v-if="hasRemovableDriver" size="sm" variant="text" :loading="phase === 'uninstall'" :disabled="!canUninstall" @click="changeDriver('uninstall')">{{ status?.state === 'pending' ? text.cleanup : text.uninstall }}</MButton>
        </template>
      </MItem>
    </MGroup>
    <MProgress v-if="phase" :label="text.phases[phase]" />
    <MFooter v-if="notice">{{ notice }}</MFooter>
  </MSection>
</template>

<script setup>
import { computed, onActivated, onMounted } from 'vue'
import { MSection, MGroup, MItem, MButton, MFooter, MProgress } from 'foundation-ui/vue'
import { ElMessageBox } from '../../composables/notifyMessage.js'
import { audioDevicesChannel, micStatusChannel } from '../../composables/deviceStatusStore.js'
import { useDeviceComponentOperation } from '../../composables/useDeviceComponentOperation.js'
import { useVbCableComponent } from '../../composables/useVbCableComponent.js'
import { useI18n } from '../../desktop/i18n/index.js'
import { vbCable } from '../../tauri-adapter.js'

const props = defineProps({ installLabel: { type: String, default: '' } })
const { t } = useI18n()
const text = computed(() => t.value.deviceHub.vbComponent)
const shared = useDeviceComponentOperation()
const { status, archiveReady, phase, notice, busy, canInstall, canEnable, hasRemovableDriver, canUninstall, check, download, changeDriver } = useVbCableComponent({
  api: vbCable,
  text,
  shared,
  confirm: (action) => {
    const label = {
      install: props.installLabel || text.value.install,
      enable: text.value.repair,
      uninstall: status.value?.state === 'pending' ? text.value.cleanup : text.value.uninstall,
    }[action]
    return ElMessageBox.confirm(text.value[`${action}Confirm`], label,
      { type: 'warning', danger: action === 'uninstall', confirmButtonText: label })
  },
  invalidate: () => { audioDevicesChannel.invalidate(); micStatusChannel.invalidate() },
})
const deviceText = computed(() => status.value ? text.value.states[status.value.state] || text.value.states.unavailable : text.value.notChecked)
const archiveText = computed(() => archiveReady.value ? text.value.packageReady : text.value.packageMissing)
shared.onOtherChange(() => { void check() })
onMounted(check)
onActivated(check)
</script>
