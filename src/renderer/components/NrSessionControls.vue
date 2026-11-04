<template>
  <section class="live-controls" :aria-busy="busy || pending" @pointerdown.capture="beginEdit" @keydown.capture="beginEdit" @focusout="cancelEdit">
    <div class="live-control-row">
      <div>
        <strong>{{ text.liveTitle }}</strong>
        <p>{{ text.liveHint }}</p>
      </div>
      <el-switch
        :model-value="pipeline.nr_requested_enabled"
        :aria-label="text.liveTitle"
        :disabled="!canChange"
        :loading="busy || pending"
        @change="(enabled) => send({ enabled }, pipeline.id)"
      />
    </div>
    <template v-if="pipeline.nr_live_controls_version >= 2">
      <div class="live-scale">
        <span>{{ text.scale }}<strong>{{ draft.scale }}%</strong></span>
        <MSlider
          :model-value="draft.scale"
          :min="20"
          :max="100"
          :step="5"
          unit="%"
          :label="text.scale"
          :disabled="!canChange || !pipeline.nr_requested_enabled"
          @update:model-value="draft.scale = $event"
          @commit="commit('scale')"
        />
      </div>
      <p class="quality-note">{{ text.scaleHint }}</p>
      <div class="live-scale">
        <span>{{ overlayText.intensity }}<strong>{{ draft.intensity }}%</strong></span>
        <MSlider
          :model-value="draft.intensity"
          :min="0"
          :max="100"
          :step="5"
          unit="%"
          :label="overlayText.intensity"
          :disabled="!canChange"
          @update:model-value="draft.intensity = $event"
          @commit="commit('intensity')"
        />
      </div>
      <p class="quality-note">{{ overlayText.applyHint }}</p>
      <MSection :title="overlayText.advanced">
        <MGroup>
          <template v-if="pipeline.nr_live_controls_version >= 3">
            <MItem icon="palette" :title="overlayText.style" stack>
              <template #control><MSeg :model-value="pipeline.nr_requested_style ?? 0" :label="overlayText.style" :options="STYLES" :disabled="!canChange" @update:model-value="(style) => send({ style: Number(style) }, pipeline.id)" /></template>
            </MItem>
            <MSliderItem :model-value="draft.skin" icon="face" :title="overlayText.skinStructure" :min="0" :max="100" :step="5" :decimals="0" unit="%" :disabled="!canChange" @update:model-value="draft.skin = $event" @commit="commit('skin')" />
            <MSwitchItem :model-value="!!pipeline.nr_requested_auto_mask" icon="auto_awesome_mosaic" :title="overlayText.autoMask" :disabled="!canChange" @update:model-value="(autoMask) => send({ autoMask }, pipeline.id)" />
          </template>
          <MSwitchItem :model-value="!!pipeline.nr_requested_ui_correction" icon="dashboard" :title="overlayText.uiCorrection" :disabled="!canChange" @update:model-value="(uiCorrection) => send({ uiCorrection }, pipeline.id)" />
          <MItem icon="motion_blur" :title="overlayText.motionQuality" stack>
            <template #control><MSeg :model-value="pipeline.nr_requested_motion_quality ?? 0" :label="overlayText.motionQuality" :options="motionOptions" :disabled="!canChange" @update:model-value="(quality) => send({ motionQuality: Number(quality) }, pipeline.id)" /></template>
          </MItem>
        </MGroup>
      </MSection>
      <MButton variant="text" icon="restart_alt" :disabled="!canChange" @click="reset">{{ overlayText.resetDefaults }}</MButton>
    </template>
    <p v-else class="quality-note">{{ overlayText.updateHost }}</p>
    <p v-if="pending" class="quality-note" role="status">
      {{ overlayText.states[state] }}
    </p>
    <p
      v-if="pipeline.nr_settings_failure_reason"
      class="quality-warning"
      role="status"
    >
      {{ overlayText.settingsFailed }}
    </p>
    <p class="quality-note" aria-live="polite">{{ saved ? overlayText.saved : overlayText.remembered }}</p>
  </section>
</template>
<script setup>
import { computed, reactive, ref, watch } from 'vue'
import { MSlider, MSliderItem, MSwitchItem, MSeg, MItem, MGroup, MSection, MButton } from 'foundation-ui/vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from '../desktop/i18n/index.js'
import { enhancementControlsText } from '../composables/enhancementControls.js'
import { nrOverlayText } from '../composables/nrOverlayMessages.js'
import { nrOverlayState } from '../composables/nrOverlayState.js'
import { nrSessionRequest, nrResetRequest } from '../composables/nrSessionControls.js'
const props = defineProps({
  pipeline: { type: Object, required: true },
  online: Boolean
})
const emit = defineEmits(['changed', 'error'])
const { locale } = useI18n()
const text = computed(() => enhancementControlsText(locale.value))
const overlayText = computed(() => nrOverlayText(locale.value))
const STYLES = [0, 1, 2, 3, 4].map((value) => ({ value, label: String(value) }))
const motionOptions = computed(() => overlayText.value.motionOptions.map((label, value) => ({ value, label })))
const busy = ref(false),
  saved = ref(false),
  editTarget = ref(null)
// the sliders show a draft while they are dragged; it is sent on release, to the stream the drag began on
const draft = reactive({ scale: 100, intensity: 100, skin: 0 })
const state = computed(() => nrOverlayState(props.pipeline, props.online))
const pending = computed(() =>
  ['warming_up', 'stopping', 'scaling'].includes(state.value)
)
const canChange = computed(
  () =>
    props.online &&
    props.pipeline.nr_toggle_supported &&
    !busy.value &&
    !pending.value
)
const requested = () => ({
  scale: props.pipeline.nr_requested_scale_percent ?? props.pipeline.nr_scale_percent ?? 100,
  intensity: Math.round((props.pipeline.nr_requested_intensity ?? 1) * 100),
  skin: Math.round((props.pipeline.nr_requested_skin_structure_strength ?? 0) * 100),
})
watch(
  () => [props.pipeline.id, ...Object.values(requested())],
  ([id], previous) => {
    if (previous && previous[0] !== id) { editTarget.value = null; saved.value = false }
    if (editTarget.value === null) Object.assign(draft, requested())
  },
  { immediate: true }
)
function beginEdit() {
  if (canChange.value && editTarget.value === null)
    editTarget.value = props.pipeline.id
}
function cancelEdit() {
  editTarget.value = null
  Object.assign(draft, requested())
}
const PATCH = {
  scale: (v) => ({ scalePercent: v }),
  intensity: (v) => ({ intensity: v / 100 }),
  skin: (v) => ({ skinStructureStrength: v / 100 }),
}
async function commit(kind) {
  const target = editTarget.value
  if (target === null) return cancelEdit()
  await send(PATCH[kind](draft[kind]), target)
  cancelEdit()
}
const reset = () => run(() => nrResetRequest(props.pipeline, props.pipeline.id, props.online))
const send = (patch, target) => run(() => nrSessionRequest(props.pipeline, target, patch, props.online))
// every change is remembered for the next streams, as the overlay does (nr_live_remember); the app's own settings come first
async function run(build) {
  if (busy.value) return
  try {
    const request = build()
    busy.value = true
    saved.value = false
    emit('error', '')
    await invoke('nr_live_set_enabled', request)
    emit('changed')
    try { await invoke('nr_live_remember', { id: request.id }); saved.value = true }
    catch { emit('error', overlayText.value.saveFailed) }
  } catch (error) {
    emit('error', error instanceof Error ? error.message : String(error))
  } finally {
    busy.value = false
  }
}
</script>
