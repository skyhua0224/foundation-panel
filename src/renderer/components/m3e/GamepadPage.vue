<template>
  <MPage :title="t.m3e.input.gamepad" :lead="t.m3e.input.gamepadNote" :eyebrow="t.m3e.input.title" back :back-label="t.m3e.back" @back="emit('back')">
    <GamepadTypePicker @ds5-selection-change="ds5Selected = $event" />
    <DualSenseSettings
      v-if="ds5Selected"
      @open-controller-meta="emit('open-controller-meta')"
      @manage-components="emit('manage-components')"
    />
    <MSection :title="t.deviceHub.controllerAdvanced">
      <MGroup>
        <MItem action icon="tune" :title="t.deviceHub.controllerAdvanced" :text="advancedOpen ? t.dualSense.tuning.collapse : t.dualSense.tuning.expand" :aria-expanded="advancedOpen" @click="toggleEmulation">
          <template #trail><MIcon :name="advancedOpen ? 'keyboard_arrow_up' : 'keyboard_arrow_down'" /></template>
        </MItem>
      </MGroup>
      <AdvancedControllerOptions v-if="advancedOpen" />
    </MSection>
    <AdvancedWarningDialog :show="warningOpen" scope="emulation" @cancel="warningOpen = false" @confirm="confirmWarning" />
  </MPage>
</template>

<script setup>
import { ref } from 'vue'
import { MPage, MSection, MGroup, MItem, MIcon } from 'foundation-ui/vue'
import DualSenseSettings from '../DualSenseSettings.vue'
import AdvancedWarningDialog from '../dualsense/AdvancedWarningDialog.vue'
import AdvancedControllerOptions from '../controllersHub/AdvancedControllerOptions.vue'
import GamepadTypePicker from '../controllersHub/GamepadTypePicker.vue'
import { isAdvancedWarningHidden, panelStorage, setAdvancedWarningHidden } from '../../composables/advancedWarning.js'
import { useI18n } from '../../desktop/i18n/index.js'

const emit = defineEmits(['back', 'open-controller-meta', 'manage-components'])
const { t } = useI18n()
const ds5Selected = ref(false)
const advancedOpen = ref(false)
const warningOpen = ref(false)

// Opening the DS4 / motion / DSU group asks first unless the warning was switched off; closing never asks.
function toggleEmulation() {
  if (advancedOpen.value) {
    advancedOpen.value = false
    return
  }
  if (isAdvancedWarningHidden(panelStorage())) {
    advancedOpen.value = true
    return
  }
  warningOpen.value = true
}
function confirmWarning(dontShowAgain) {
  if (dontShowAgain) setAdvancedWarningHidden(panelStorage(), true)
  warningOpen.value = false
  advancedOpen.value = true
}
</script>
