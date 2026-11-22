<template>
  <section class="live-controls" :aria-busy="busy || pending" @pointerdown.capture="pressEdit" @keydown.capture="pressEdit" @focusout="(event) => { if (focusLeaves(event.currentTarget, event.relatedTarget)) cancelEdit() }">
    <div v-if="showGlobal" class="live-control-row">
      <MSwitchItem
        :model-value="globalEnabled === true"
        icon="wand_stars"
        :title="text.liveTitle"
        :text="text.nrPermissionHint"
        :disabled="!online || globalEnabled == null || busy"
        @update:model-value="setGlobal"
      >
        <template #extra><MLoading v-if="busy || pending" size="18px" /></template>
      </MSwitchItem>
    </div>
    <template v-if="pipeline.nr_live_controls_version >= 2">
      <MGroup>
        <MSliderItem :model-value="draft.scale" icon="aspect_ratio" :title="text.scale" :text="text.scaleHint" :min="20" :max="100" :step="5" :decimals="0" unit="%" size="m" icon-start="photo_size_select_small" icon-end="photo_size_select_large" :disabled="!canChange || !pipeline.nr_requested_enabled" @update:model-value="draft.scale = $event" @commit="commit('scale')" />
        <MSliderItem :model-value="draft.intensity" icon="tune" :title="overlayText.intensity" :text="overlayText.applyHint" :min="0" :max="100" :step="5" :decimals="0" unit="%" size="m" icon-start="remove" icon-end="add" :disabled="!canChange" @update:model-value="draft.intensity = $event" @commit="commit('intensity')" />
      </MGroup>
      <MSection :title="overlayText.advanced">
        <MGroup>
          <template v-if="pipeline.nr_live_controls_version >= 3">
            <MItem icon="palette" :title="overlayText.style" stack>
              <template #control><MSeg :model-value="pipeline.nr_requested_style ?? 0" :label="overlayText.style" :options="STYLES" :disabled="!canChange" @update:model-value="(style) => send({ style: Number(style) }, pipeline.id)" /></template>
            </MItem>
            <MSliderItem :model-value="draft.skin" icon="face" :title="overlayText.skinStructure" :min="0" :max="100" :step="5" :decimals="0" unit="%" size="m" icon-start="remove" icon-end="add" :disabled="!canChange" @update:model-value="draft.skin = $event" @commit="commit('skin')" />
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
    <p v-if="state === 'degraded'" class="quality-warning" role="status">
      {{ degradedReason.main }}<br v-if="degradedReason.detail">{{ degradedReason.detail }}
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
import { MLoading, MSliderItem, MSwitchItem, MSeg, MItem, MGroup, MSection, MButton } from 'foundation-ui/vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from '../desktop/i18n/index.js'
import { enhancementControlsText } from '../composables/enhancementControls.js'
import { nrDegradedReason, nrOverlayText } from '../composables/nrOverlayMessages.js'
import { nrOverlayState } from '../composables/nrOverlayState.js'
import { nrSessionRequest, nrResetRequest, focusLeaves, startsSliderEdit } from '../composables/nrSessionControls.js'
const props = defineProps({
  pipeline: { type: Object, required: true },
  online: Boolean,
  globalEnabled: { type: Boolean, default: null },
  showGlobal: { type: Boolean, default: true },
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
const degradedReason = computed(() => nrDegradedReason(props.pipeline, overlayText.value))
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
const pressEdit = (event) => { if (startsSliderEdit(event)) beginEdit() }
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
async function setGlobal(enabled) {
  if (busy.value) return
  busy.value = true
  try {
    const savedEnabled = await invoke('set_global_nr_enabled', { enabled })
    emit('changed', savedEnabled)
  }
  catch (e) { emit('error', String(e)) }
  finally { busy.value = false }
}
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

<style>
/* the live controls on 画质增强 › 当前会话 and on 当前串流: every row is a label with its value beside it, a gap between them */
.live-controls { display: grid; gap: 4px; padding: 4px 0 8px; }
.live-controls .live-control-row { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; margin-bottom: 8px; }
.live-controls .live-control-row strong { font: var(--m-title-small); color: var(--md-sys-color-on-surface); }
.live-controls .live-control-row p { margin: 4px 0 0; font: var(--m-body-medium); color: var(--md-sys-color-on-surface-variant); }
.live-controls .quality-note, .live-controls .quality-warning { margin: 0 0 8px; font: var(--m-body-small); color: var(--md-sys-color-on-surface-variant); }
.live-controls .quality-warning { color: var(--md-sys-color-error); }
</style>
