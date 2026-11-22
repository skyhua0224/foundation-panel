<template>
  <MSection :title="t.deviceHub.components.inputDrivers">
    <MSkeleton v-if="!initialized" :rows="2" />
    <template v-else>
      <MGroup>
        <MItem icon="gamepad" :tone="!probeFailed.vigem && vigemStatus.installed ? 'success' : ''" :title="t.controllersHub.peripherals.vigem.title">
          <template #trail>
            <MChip :tone="!probeFailed.vigem && vigemStatus.installed ? 'ok' : 'neutral'">{{ probeFailed.vigem ? t.deviceHub.probeUnavailable : (vigemStatus.installed ? (vigemStatus.version ? 'v' + vigemStatus.version : t.controllersHub.peripherals.installed) : t.controllersHub.peripherals.notInstalled) }}</MChip>
            <MButton size="sm" :variant="vigemStatus.installed ? 'tonal' : 'filled'" :loading="ops.vigem" :disabled="refreshing || probeFailed.vigem" @click="confirmToggle('vigem')">{{ vigemStatus.installed ? t.controllersHub.peripherals.uninstall : t.controllersHub.peripherals.install }}</MButton>
          </template>
        </MItem>
        <MItem v-if="vmouseSupported" icon="mouse" :tone="!probeFailed.vmouse && vmouseStatus.installed ? 'success' : ''" :title="t.controllersHub.peripherals.vmouse.title" :text="vmouseNotice">
          <template #trail>
            <MChip :tone="!probeFailed.vmouse && vmouseStatus.installed ? 'ok' : 'neutral'">{{ probeFailed.vmouse ? t.deviceHub.probeUnavailable : (vmouseStatus.installed ? t.controllersHub.peripherals.installed : t.controllersHub.peripherals.notInstalled) }}</MChip>
            <MButton size="sm" :variant="vmouseStatus.installed ? 'tonal' : 'filled'" :loading="ops.vmouse" :disabled="refreshing || probeFailed.vmouse" @click="confirmToggle('vmouse')">{{ vmouseStatus.installed ? t.controllersHub.peripherals.uninstall : t.controllersHub.peripherals.install }}</MButton>
          </template>
        </MItem>
        <MItem v-if="vmouseSupported" icon="toggle_on" :title="t.stream.vmouseToggle">
          <template #trail><MButton size="sm" :variant="vmouseEnabled ? 'filled' : 'outlined'" :loading="vmouseConfigSaving" :disabled="refreshing || !vmouseStatusKnown" @click="toggleVmouse">{{ !vmouseStatusKnown ? t.stream.vmouseUnknown : vmouseEnabled ? t.stream.vmouseOn : t.stream.vmouseOff }}</MButton></template>
        </MItem>
      </MGroup>
      <!-- a driver being installed or removed: the wavy progress under the list (the button keeps its own busy state) -->
      <MProgress v-for="d in visibleDrivers.filter((k) => ops[k])" :key="d" class="input-drivers__busy" :label="busyLabel(d)" />
    </template>
  </MSection>
</template>

<script setup>
import { MSection, MGroup, MItem, MChip, MButton, MSkeleton, MProgress } from 'foundation-ui/vue'
import { computed, onMounted } from 'vue'
import { ElMessageBox } from '../../composables/notifyMessage.js'
import { useInputDrivers } from '../../composables/useInputDrivers.js'
import { formatMessage } from '../../shared/format-message.js'
import { useI18n } from '../../desktop/i18n/index.js'

const { t } = useI18n()

const {
  vigemStatus, vmouseStatus, probeFailed, ops, initialized, refreshing,
  refreshAll, installVigem, uninstallVigem,
  installVmouse, uninstallVmouse,
  vmouseSupported, vmouseEnabled, vmouseStatusKnown, vmouseNotice, vmouseConfigSaving, toggleVmouse,
} = useInputDrivers(t)

const visibleDrivers = computed(() => vmouseSupported.value ? ['vigem', 'vmouse'] : ['vigem'])

// the label of the wave while a driver is installed or removed (the status is still the one before the operation)
const busyLabel = (d) => { const p = t.value.controllersHub.peripherals; const installed = d === 'vigem' ? vigemStatus.installed : vmouseStatus.value.installed; return formatMessage(installed ? p.uninstallingDriver : p.installingDriver, { name: p[d].title }) }

async function confirmToggle(tool) {
  const strings = t.value.controllersHub.peripherals[tool]
  const installed = tool === 'vigem' ? vigemStatus.installed : vmouseStatus.value.installed
  const action = installed ? 'uninstall' : 'install'
  try {
    await ElMessageBox.confirm(strings[action === 'install' ? 'confirmInstall' : 'confirmUninstall'], t.value.controllersHub.peripherals.confirmTitle, {
      type: action === 'uninstall' ? 'warning' : 'info',
      // removing a driver is destructive: the error-colour button with the verb
      ...(action === 'uninstall' ? { danger: true, confirmButtonText: t.value.systemTools.uninstall } : {}),
    })
  } catch {
    return
  }
  if (tool === 'vigem') {
    await (installed ? uninstallVigem() : installVigem())
  } else {
    await (installed ? uninstallVmouse() : installVmouse())
  }
}

onMounted(refreshAll)
</script>
