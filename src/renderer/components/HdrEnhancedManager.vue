<template>
  <MPage :title="current === 'session' ? text.session : current === 'keys' ? text.keys : current === 'components' ? text.components : text.title" :back="!!current" :back-label="t.m3e.back" @back="goBack">
    <MBanner v-if="error" tone="error" icon="error">{{ enhancementError(text, error) }}</MBanner>

    <!-- Overlay: the two controls of the page, then where the rest lives -->
    <template v-if="!current">
      <MGroup>
        <MSwitchItem :model-value="visible" icon="picture_in_picture" :title="text.show" :disabled="saving || !loaded" @update:model-value="setVisibility" />
        <MSliderItem v-model="draftOpacity" icon="opacity" :title="text.opacity" :min="35" :max="95" :step="1" :decimals="0" unit="%" size="m" icon-start="brightness_medium" icon-end="brightness_6" :disabled="saving || !loaded" @commit="save({ opacity: draftOpacity })" />
      </MGroup>
      <div class="quality-manager m-quality-preview" aria-hidden="true"><div class="preview-stage">
        <div class="mini-overlay" :style="{ background: `color-mix(in srgb, var(--md-sys-color-surface-container-highest) ${draftOpacity}%, transparent)` }">
          <header><Rank aria-hidden="true" /><i aria-hidden="true" /> DLSS NR <ArrowUp aria-hidden="true" /></header>
          <div class="mini-body">
            <div>{{ text.enhancement }}<span class="mini-toggle" /></div>
            <p>{{ text.previewState }}</p>
            <footer><kbd>{{ nrShortcutLabel(registration) || text.disabledKey }}</kbd><span>{{ text.action }}</span></footer>
          </div>
        </div>
      </div></div>
      <MSection :title="text.keys">
        <MGroup>
          <MItem action icon="visibility" :title="text.visibilityKey" @click="open('keys')"><template #trail><kbd class="m-quality-kbd">{{ displayShortcut(settings.overlayShortcut) || text.unset }}</kbd></template></MItem>
          <MItem action icon="wand_stars" :title="text.nrKey" @click="open('keys')"><template #trail><kbd class="m-quality-kbd">{{ nrShortcutLabel(registration) || text.unset }}</kbd></template></MItem>
        </MGroup>
      </MSection>
      <MGroup>
        <MItem action icon="cast_connected" :title="text.session" :text="pipelines.length ? `${text.stream} #${pipelines[0].id}` : text.noSession" @click="open('session')" />
        <MItem action icon="extension" :title="text.components" @click="open('components')" />
      </MGroup>
      <MFooter>{{ saved ? text.saved : text.local }}</MFooter>
    </template>

    <!-- Current session -->
    <template v-else-if="current === 'session'">
      <MBanner v-if="!online" tone="warn" icon="warning">{{ text.disconnected }}<template #actions><MButton variant="text" size="sm" @click="pollOnce">{{ text.refresh }}</MButton></template></MBanner>
      <MEmpty v-else-if="!pipelines.length" icon="cast_connected" :title="text.noSession" />
      <template v-else>
        <MGroup>
          <MChoiceItem v-if="pipelines.length > 1 || !pipeline" icon="cast_connected" :title="text.choose" :model-value="selectedId ?? ''" :options="pipelines.map((p) => ({ value: p.id, label: `${text.stream} #${p.id} · ${nrOutputLabel(p)}` }))" :disabled="saving" :cancel-label="text.cancel" @update:model-value="(v) => selectSession(Number(v))" />
          <MItem v-else icon="cast_connected" :title="`${text.stream} #${pipeline.id}`" :value="text.connected" />
          <template v-if="pipeline">
            <MItem icon="wand_stars" :title="text.enhancement" :value="overlayText.states[nrOverlayState(pipeline, online)]" />
            <MItem icon="monitor" :title="text.output" :value="nrOutputLabel(pipeline)" />
          </template>
        </MGroup>
        <MNotice v-if="selectedId && !pipeline" tone="warn">{{ text.expired }}</MNotice>
        <MBanner v-if="pipeline?.dv_profile && pipeline.dv_state !== 'active'" tone="info" icon="info">{{ pipeline.dv_state === 'waiting' ? overlayText.dvWaiting : overlayText.dvFallback }}</MBanner>
        <NrSessionControls v-if="pipeline" :pipeline="pipeline" :online="online" @changed="pollOnce" @error="error = $event" />
        <MButton v-if="pipeline" icon="monitor" :disabled="saving" @click="setVisibility(true)">{{ text.open }}</MButton>
      </template>
      <MFooter>{{ text.sessionHint }}</MFooter>
    </template>

    <!-- Shortcuts: press a key while a row records -->
    <div v-else-if="current === 'keys'" class="m-quality-keys" @keydown="captureKey">
      <MBanner v-if="recording" tone="info" icon="keyboard">{{ text.recordHint }}<template #actions><MButton variant="text" size="sm" @click="cancelRecording">{{ text.cancel }}</MButton></template></MBanner>
      <MSection v-for="group in shortcutGroups" :key="group.label" :title="group.label">
        <MGroup>
          <template v-for="item in group.inputs" :key="item.key">
            <MChoiceItem v-if="item.gamepad" icon="sports_esports" :title="text.gamepad" :model-value="settings[item.key] || ''" :options="[{ value: '', label: text.unset }, ...FIXED_GAMEPAD_SHORTCUTS.map((o) => ({ value: o, label: displayGamepadShortcut(o) }))]" :disabled="saving || !loaded || !registration.gamepadSupported" :cancel-label="text.cancel" @update:model-value="(v) => save({ [item.key]: v })" />
            <MItem v-else action icon="keyboard" :title="text.keyboard" :value="recording === item.key ? text.record : displayShortcut(settings[item.key]) || text.unset" :disabled="saving || !loaded" @click="startRecording(item.key)">
              <template v-if="settings[item.key]" #trail><MIconButton icon="close" :label="text.clear" :disabled="saving || !loaded" @click.stop="clearShortcut(item.key)" /></template>
            </MItem>
          </template>
        </MGroup>
        <MFooter v-for="item in group.inputs.filter((i) => !i.gamepad && settings[i.key] && !registration[i.registered])" :key="item.key">{{ text.statusUnavailable }}</MFooter>
      </MSection>
      <MFooter>{{ text.keyHint }} {{ text.gamepadHint }}</MFooter>
      <MFooter v-if="loaded && !registration.gamepadSupported">{{ text.gamepadUnsupported }}</MFooter>
      <MButton variant="text" icon="restart_alt" :disabled="saving || !loaded" @click="resetShortcuts">{{ text.reset }}</MButton>
    </div>

    <!-- Components -->
    <template v-else-if="current === 'components'">
      <div class="enhancement-stack"><EnhancementManager kind="nr" /><EnhancementManager kind="hdr" /></div>
      <MFooter>{{ text.componentHint }}</MFooter>
    </template>
  </MPage>
</template>
<script setup>
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { ArrowUp, Rank } from '@element-plus/icons-vue'
import { MPage, MGroup, MSection, MItem, MSwitchItem, MSliderItem, MChoiceItem, MBanner, MNotice, MEmpty, MFooter, MButton, MIconButton, useSubpage } from 'foundation-ui/vue'
import { displayGamepadShortcut, FIXED_GAMEPAD_SHORTCUTS } from '../composables/controllerButtons.js'
import NrSessionControls from './NrSessionControls.vue'
import EnhancementManager from './EnhancementManager.vue'
import { useI18n } from '../desktop/i18n/index.js'
import {
  DEFAULT_NR_SHORTCUT,
  displayShortcut,
  nrShortcutLabel,
  shortcutFromEvent,
  enhancementControlsText,
  enhancementError
} from '../composables/enhancementControls.js'
import {
  nrOutputLabel,
  nrOverlayState,
  nrPipelines
} from '../composables/nrOverlayState.js'
import { shortcutCapture } from '../composables/shortcutCapture.js'
import { nrOverlayText } from '../composables/nrOverlayMessages.js'
// Element Plus has no keyboard glyph; use its established input icon.
import { EditPen as Keyboard } from '@element-plus/icons-vue'
const { locale } = useI18n()
const text = computed(() => enhancementControlsText(locale.value)),
  overlayText = computed(() => nrOverlayText(locale.value))
const t = useI18n().t
const { current, open, back } = useSubpage('quality')
const settings = ref({ nrShortcut: '', overlayShortcut: '', nrGamepad: '', overlayGamepad: '', opacity: 62 }),
  registration = ref({})
const draftOpacity = ref(62),
  loaded = ref(false),
  saving = ref(false),
  visible = ref(false),
  saved = ref(false),
  error = ref(''),
  recording = ref('')
const pipelines = ref([]),
  selectedId = ref(null),
  online = ref(false)
const pipeline = computed(() =>
  pipelines.value.find((p) => p.id === selectedId.value)
)
const shortcutGroups = computed(() => [
  { label: text.value.visibilityKey, inputs: [
    { key: 'overlayShortcut', registered: 'overlayRegistered' },
    { key: 'overlayGamepad', gamepad: true }
  ] },
  { label: text.value.nrKey, inputs: [
    { key: 'nrShortcut', registered: 'nrRegistered' },
    { key: 'nrGamepad', gamepad: true }
  ] }
])
let disposed = false,
  polling = false,
  timer,
  unlisten = []
function applyStatus(status) {
  if (disposed || !status?.settings) return
  settings.value = status.settings
  registration.value = status
  draftOpacity.value = status.settings.opacity
  selectedId.value = status.target
  loaded.value = true
}
async function save(patch) {
  if (saving.value) return
  saving.value = true
  error.value = ''
  saved.value = false
  try {
    applyStatus(await invoke('nr_overlay_save_settings', { patch }))
    saved.value = true
  } catch (e) {
    error.value = String(e)
    try {
      applyStatus(await invoke('nr_overlay_settings'))
    } catch {}
  } finally {
    saving.value = false
  }
}
const capture = shortcutCapture(
  active => invoke('nr_overlay_capture_shortcut', { active }),
  key => { recording.value = key }
)
async function cancelRecording() {
  try { await capture.cancel() } catch (e) { if (!disposed) error.value = String(e) }
}
async function startRecording(key) {
  error.value = ''
  try { await capture.start(key) } catch (e) { if (!disposed) error.value = String(e) }
}
async function captureKey(event) {
  if (!recording.value) return
  const result = shortcutFromEvent(event)
  if (event.key !== 'Tab') event.preventDefault()
  if (result.cancel) {
    await cancelRecording()
    return
  }
  if (result.modifier) return
  if (result.error) {
    error.value = result.error
    return
  }
  const key = recording.value
  await cancelRecording()
  await save({ [key]: result.value })
}
async function clearShortcut(key) {
  await cancelRecording()
  await save({ [key]: '' })
}
async function resetShortcuts() {
  await cancelRecording()
  await save({ nrShortcut: DEFAULT_NR_SHORTCUT, overlayShortcut: '', nrGamepad: '', overlayGamepad: '' })
}
function goBack() {
  void cancelRecording()
  back()
}
async function setVisibility(value) {
  try {
    await invoke('nr_overlay_set_visible', { visible: value })
    visible.value = value
    error.value = ''
  } catch (e) {
    error.value = String(e)
  }
}
async function selectSession(id) {
  try {
    await invoke('nr_overlay_select_session', { id })
    selectedId.value = id
    error.value = ''
  } catch (e) {
    error.value = String(e)
  }
}
async function pollOnce() {
  if (disposed || polling) return
  polling = true
  try {
    const result = await invoke('nr_live_status')
    if (!disposed) {
      pipelines.value = nrPipelines(result?.pipelines)
      online.value = true
      if (
        loaded.value &&
        selectedId.value === null &&
        pipelines.value.length === 1
      )
        await selectSession(pipelines.value[0].id)
    }
  } catch {
    if (!disposed) {
      online.value = false
      pipelines.value = []
    }
  }
  try {
    const value = await invoke('nr_overlay_is_visible')
    if (!disposed) visible.value = value
  } catch {}
  polling = false
}
async function poll() {
  await pollOnce()
  if (!disposed) timer = setTimeout(poll, 1200)
}
onMounted(async () => {
  let settingsRevision = 0
  window.addEventListener('blur', cancelRecording)
  for (const [name, handler] of [
    ['nr-settings-changed', (e) => { settingsRevision++; applyStatus(e.payload) }],
    [
      'nr-overlay-visibility',
      (e) => {
        visible.value = e.payload
      }
    ],
    [
      'nr-action-error',
      (e) => {
        error.value = e.payload
      }
    ]
  ]) {
    try {
      const stop = await listen(name, handler)
      if (disposed) stop()
      else unlisten.push(stop)
    } catch {}
  }
  try {
    const revision = settingsRevision
    const status = await invoke('nr_overlay_settings')
    if (revision === settingsRevision) applyStatus(status)
  } catch (e) {
    error.value = String(e)
  }
  if (!disposed) {
    void poll()
  }
})
onUnmounted(() => {
  disposed = true
  clearTimeout(timer)
  void cancelRecording()
  unlisten.forEach((stop) => stop())
  window.removeEventListener('blur', cancelRecording)
})
</script>

<style src="../styles/EnhancementManager.css"></style>
<style>
.m-quality-preview { margin: 4px 0; padding: 0; width: 100%; max-width: none; border-radius: 28px; overflow: hidden; }
.m-quality-preview svg { width: 14px; height: 14px; flex: none; }
/* the preview is drawn in the colours of the theme (the real overlay is dark over a game; here it is a sketch of its parts) */
.m-quality-preview .preview-stage { min-height: 0; height: auto; padding: 20px; border-radius: 28px; width: 100%; background: var(--md-sys-color-surface-container-high); color: var(--md-sys-color-on-surface); }
.m-quality-preview .mini-overlay { width: min(100%, 380px); border: 0; border-radius: 16px; color: var(--md-sys-color-on-surface); box-shadow: 0 1px 2px color-mix(in srgb, var(--md-sys-color-shadow) 18%, transparent); }
.m-quality-preview .mini-overlay header { border-bottom-color: var(--md-sys-color-outline-variant); }
.m-quality-preview .mini-overlay header i { background: var(--md-sys-color-primary); border-radius: 50%; }
.m-quality-preview .mini-body p { color: var(--md-sys-color-on-surface-variant); }
.m-quality-preview .mini-body footer { border-top-color: var(--md-sys-color-outline-variant); }
.m-quality-preview .mini-toggle { border: 0; border-radius: 9999px; background: var(--md-sys-color-primary); width: 32px; height: 18px; }
.m-quality-kbd { padding: 4px 10px; border-radius: 8px; background: var(--md-sys-color-surface-container-highest); color: var(--md-sys-color-on-surface); font: 12px/16px ui-monospace, 'Cascadia Mono', Consolas, monospace; }
</style>
