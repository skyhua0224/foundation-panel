<template>
  <SettingsCard :title="t.settings.pet" icon="pets">
    <SettingsRow :name="t.settings.deskObserve" :description="t.settings.deskObserveDesc">
      <SettingsSwitch
        v-model="petEnabled"
        :label="t.settings.deskObserve"
        :disabled="visionConfirmPending"
        @change="onPetToggle"
      />
    </SettingsRow>

    <SettingsRow
      v-if="petEnabled"
      :name="t.settings.observeInterval"
      :description="t.settings.observeIntervalDesc"
      stack
    >
      <SettingsSelect
        v-model="petIntervalSec"
        :options="petIntervalOptions"
        :label="t.settings.observeInterval"
        @change="onPetIntervalChange"
      />
    </SettingsRow>

    <SettingsRow
      v-if="petEnabled"
      :name="t.settings.pokeMita"
    >
      <template #description>
        {{ t.settings.pokeMitaDesc }}
        <span v-if="pokeFailed" class="dsk-pet-error">{{ t.petTool.pokeFailed }}</span>
      </template>
      <MButton variant="tonal" icon="touch_app" :loading="isObserving" @click="poke">
        {{ isObserving ? t.settings.pokeBtnObserving : t.settings.pokeBtn }}
      </MButton>
    </SettingsRow>
  </SettingsCard>
</template>

<script setup>
import { computed, ref, watch } from 'vue'
import { MButton } from 'foundation-ui/vue'
import { useDesktopPet } from '../../../composables/useDesktopPet.js'
import { requestConfirm } from '../../composables/useConfirm.js'
import { useI18n } from '../../i18n/index.js'
import SettingsCard from './SettingsCard.vue'
import SettingsRow from './SettingsRow.vue'
import SettingsSelect from './SettingsSelect.vue'
import SettingsSwitch from './SettingsSwitch.vue'

const { t } = useI18n()

const {
  petEnabled,
  isObserving,
  observeInterval,
  pokeFailed,
  startObserving,
  stopObserving,
  setIntervalSeconds,
  poke,
} = useDesktopPet()

const petIntervalSec = ref(Math.round(observeInterval.value / 1000))
const visionConfirmPending = ref(false)
watch(observeInterval, (value) => {
  petIntervalSec.value = Math.round(value / 1000)
})

const petIntervalOptions = computed(() => [
  { value: 15, label: t.value.settings.intervals.s15 },
  { value: 30, label: t.value.settings.intervals.s30 },
  { value: 60, label: t.value.settings.intervals.s60 },
  { value: 120, label: t.value.settings.intervals.m2 },
  { value: 300, label: t.value.settings.intervals.m5 },
])

async function onPetToggle(nextValue = petEnabled.value) {
  if (!nextValue) {
    stopObserving()
    return
  }

  // v-model updates first; keep the feature off until the user confirms.
  petEnabled.value = false
  if (visionConfirmPending.value) return

  visionConfirmPending.value = true
  // The window's own confirm dialog (a pad can answer it); screenshots leave the machine, so the answer is asked every time.
  const consent = t.value.petTool.visionPrivacyConfirm
  const accepted = await requestConfirm({
    title: consent.title,
    message: consent.message,
    confirmLabel: consent.accept,
    cancelLabel: consent.cancel,
  })
  finishVisionConfirmation(accepted)
}

function finishVisionConfirmation(accepted) {
  if (!visionConfirmPending.value) return
  visionConfirmPending.value = false
  if (accepted) startObserving()
}

function onPetIntervalChange(nextValue = petIntervalSec.value) {
  setIntervalSeconds(nextValue)
}
</script>

<style scoped>
.dsk-pet-error { display: block; margin-top: 4px; color: var(--md-sys-color-error); }
</style>
