<template>
  <GamepadTypePicker @ds5-selection-change="ds5Selected = $event" />
  <DualSenseSettings
    v-if="ds5Selected"
    @open-controller-meta="emit('open-controller-meta')"
    @manage-components="emit('manage-components')"
  />
  <div class="chub-advanced-block">
    <el-collapse v-model="controllerAdvancedOpen" class="chub-advanced" :before-collapse="beforeAdvancedToggle">
      <el-collapse-item name="advanced" :title="t.deviceHub.controllerAdvanced">
        <AdvancedControllerOptions />
      </el-collapse-item>
    </el-collapse>
  </div>
  <AdvancedWarningDialog
    :show="advancedWarningOpen"
    scope="emulation"
    @cancel="advancedWarningOpen = false"
    @confirm="confirmAdvancedWarning"
  />
</template>

<script setup>
import { ref } from 'vue'
import DualSenseSettings from '../DualSenseSettings.vue'
import AdvancedWarningDialog from '../dualsense/AdvancedWarningDialog.vue'
import { isAdvancedWarningHidden, panelStorage, setAdvancedWarningHidden } from '../../composables/advancedWarning.js'
import AdvancedControllerOptions from './AdvancedControllerOptions.vue'
import GamepadTypePicker from './GamepadTypePicker.vue'
import { useI18n } from '../../desktop/i18n/index.js'

const emit = defineEmits(['open-controller-meta', 'manage-components'])
const { t } = useI18n()
const ds5Selected = ref(false)
const controllerAdvancedOpen = ref([])
const advancedWarningOpen = ref(false)

// Opening the section asks first unless the warning was switched off; closing never asks.
// Returning false keeps el-collapse from opening it on its own.
function beforeAdvancedToggle() {
  if (controllerAdvancedOpen.value.includes('advanced') || isAdvancedWarningHidden(panelStorage())) return true
  advancedWarningOpen.value = true
  return false
}

function confirmAdvancedWarning(dontShowAgain) {
  if (dontShowAgain) setAdvancedWarningHidden(panelStorage(), true)
  advancedWarningOpen.value = false
  controllerAdvancedOpen.value = ['advanced']
}
</script>
