<template>
  <MPage :title="title" :back="!!current || viaDualsense" :back-label="t.m3e.back" @back="onBack">
    <template #actions>
      <MIconButton v-if="current === 'status'" icon="refresh" :label="tx('refresh')" :disabled="statusLoading" @click="refreshAll" />
      <MOverflow v-if="!current && dsVisible && tuning.snapshot.value" :items="[{ id: 'restore', label: tr('restore_defaults') }]" :label="t.deviceHub.refresh" @select="restoreOpen = true" />
    </template>

    <!-- Root: short, one row per thing -->
    <template v-if="!current">
      <MNotice v-if="gt.unreachable.value" tone="error">{{ t.controllersHub.emulation.sunshineUnreachable }}<template #action><MButton size="sm" :loading="gt.loading.value" @click="gt.loadConfig">{{ t.controllersHub.emulation.retry }}</MButton></template></MNotice>
      <MGroup v-else>
        <MChoiceItem v-model="gt.selectedMode.value" icon="sports_esports" :title="t.m3e.padType" :options="typeOptions" :disabled="gt.busyKey.value !== ''" :cancel-label="t.common.cancel" />
        <MItem action icon="apps" :title="t.controllersHub.emulation.perAppTitle" @click="open('per-game')" />
        <MItem action icon="settings_input_component" :title="t.deviceHub.controllerAdvanced" @click="askAdvanced('emulation', 'emulation')" />
      </MGroup>

      <template v-if="dsVisible">
        <MGroup>
          <MItem action icon="info" title="DualSense" :value="statusLabel" @click="open('status')" />
        </MGroup>
        <template v-if="tuning.snapshot.value">
          <MGroup>
            <MItem :title="tr('presets.title')" icon="tune" stack>
              <template #control>
                <MSeg block :model-value="tuning.selectedPreset.value" :label="tr('presets.title')" :options="presetOptions" @update:model-value="tuning.applyPreset" />
              </template>
            </MItem>
          </MGroup>
          <MSection :title="tr('title')">
            <MGroup>
              <MItem v-for="card in tuning.cards.value" :key="card.id" action :icon="CARD_ICONS[card.id] || 'tune'" :title="tr('cards.' + card.id)" @click="open('card/' + card.id)" />
              <MItem action icon="build" :title="tr('advanced')" @click="askAdvanced('advanced')"><template #trail><span class="m-item__count">{{ tuning.advancedTotal.value }}</span></template></MItem>
            </MGroup>
          </MSection>
        </template>
        <MSkeleton v-else-if="tuning.loading.value" :rows="3" />
        <MNotice v-else-if="tuning.loadError.value && !tuning.unsupported.value" tone="error" :title="tr('load_failed')">{{ tuning.loadError.value }}<template #action><MButton size="sm" @click="tuning.load">{{ tr('retry') }}</MButton></template></MNotice>
      </template>

    </template>

    <!-- Per-game types -->
    <template v-else-if="current === 'per-game'">
      <MSkeleton v-if="gt.appsLoading.value" :rows="3" />
      <MNotice v-else-if="gt.appsError.value" tone="error" :title="t.controllersHub.emulation.appLoadFailed"><template #action><MButton size="sm" @click="gt.loadApps">{{ t.controllersHub.emulation.retry }}</MButton></template></MNotice>
      <MEmpty v-else-if="!gt.apps.value.length" icon="apps" :title="t.controllersHub.emulation.noApps" />
      <MGroup v-else>
        <MChoiceItem v-for="(app, index) in gt.apps.value" :key="app.name + index" :model-value="app.gamepad || ''" :title="app.name" :options="gt.appModeOptions.value" :disabled="gt.appSavingIndex.value === index" :cancel-label="t.common.cancel" @update:model-value="(v) => gt.saveAppGamepad(index, v)" />
      </MGroup>
      <MFooter>{{ t.controllersHub.emulation.perAppHint }}</MFooter>
    </template>

    <!-- DualSense status: the facts, the tools, what to do next -->
    <template v-else-if="current === 'status'">
      <MSkeleton v-if="!status && !statusFailed" :rows="4" />
      <MNotice v-else-if="!status" tone="error">{{ tx('errors.DS5-HOST-001') }}</MNotice>
      <template v-else>
        <MGroup>
          <MItem v-for="row in statusRowsView" :key="row.id" :title="tx('status.rows.' + row.id)" :text="row.text" />
        </MGroup>
        <MNotice v-if="needsTransport" tone="warn" :title="status.state === 'usbip_missing' ? tx('status.install.title') : ''">
          <ol v-if="status.state === 'usbip_missing'" class="m-status__steps"><li v-for="(step, n) in installSteps" :key="n">{{ step }}</li></ol>
          <template #action><MButton size="sm" @click="emit('manage-components')">{{ tx('status.install.open') }}</MButton></template>
        </MNotice>
        <MSection :title="t.deviceHub.tabs.tools">
          <MGroup><MItem action icon="gamepad" :title="tr('open_meta')" @click="emit('open-controller-meta')"><template #trail><MIcon name="open_in_new" /></template></MItem></MGroup>
        </MSection>
        <MFooter>{{ nextText }}</MFooter>
      </template>
    </template>

    <!-- Tuning groups -->
    <template v-else-if="current.startsWith('card/')">
      <MGroup>
        <TuningRow v-for="key in cardRows" :key="key.id" :descriptor="key" :model-value="tuning.edited[key.id]" @update:model-value="tuning.onValue(key.id, $event)" @reset="tuning.resetKey(key.id)" />
        <TuningColor v-if="cardObj?.lighting && tuning.hasColor.value" :r="Number(tuning.edited[82])" :g="Number(tuning.edited[83])" :b="Number(tuning.edited[84])" :mode="String(tuning.edited[81])" @update="tuning.setColor" />
      </MGroup>
    </template>
    <template v-else-if="current === 'pending'">
      <MGroup><TuningRow v-for="key in tuning.pendingRows.value" :key="key.id" :descriptor="key" :model-value="tuning.edited[key.id]" disabled /></MGroup>
      <MFooter>{{ tr('pending.hint') }}</MFooter>
    </template>

    <!-- Advanced (behind the five-second warning) -->
    <template v-else-if="current === 'advanced'">
      <MField v-model="tuning.search.value" :label="tr('search')" />
      <MGroup>
        <MSwitchItem v-model="tuning.onlyModified.value" :title="tr('only_modified')" />
        <MSwitchItem v-model="tuning.showPending.value" :title="tr('show_pending')" />
      </MGroup>
      <MEmpty v-if="!tuning.groups.value.length" icon="search" :title="tr('no_matches')" />
      <MGroup v-else>
        <MItem v-if="tuning.pendingRows.value.length" action icon="schedule" :title="tr('pending.title')" @click="open('pending')"><template #trail><span class="m-item__count">{{ tuning.pendingRows.value.length }}</span></template></MItem>
        <MItem v-for="group in tuning.groups.value" :key="group.id" action :title="tr('groups.' + group.id)" @click="open('advanced/' + group.id)"><template #trail><span class="m-item__count">{{ group.wired.length + group.pending.length }}</span></template></MItem>
      </MGroup>
    </template>
    <template v-else-if="current.startsWith('advanced/')">
      <MGroup>
        <TuningRow v-for="key in advGroup?.wired || []" :key="key.id" :descriptor="key" :model-value="tuning.edited[key.id]" @update:model-value="tuning.onValue(key.id, $event)" @reset="tuning.resetKey(key.id)" />
        <TuningRow v-for="key in advGroup?.pending || []" :key="key.id" :descriptor="key" :model-value="tuning.edited[key.id]" disabled />
      </MGroup>
      <TuningTrigger v-if="advGroup?.id === 3" class="ds5t-preview" :enabled="tuning.edited[24] === true" :strength="Number(tuning.edited[25] ?? 1)" :left-strength="Number(tuning.edited[26] ?? 1)" :right-strength="Number(tuning.edited[27] ?? 1)" />
    </template>

    <!-- DS4 / motion / DSU -->
    <template v-else-if="current === 'emulation'">
      <AdvancedControllerOptions />
    </template>

    <Transition name="m-savebar">
      <div v-if="tuning.dirtyCount.value > 0 || tuning.saving.value" class="m-savebar" role="region" :aria-label="tr('save')">
        <span class="m-savebar__count">{{ tr('dirty_count', { count: tuning.dirtyCount.value }) }}</span>
        <MButton variant="text" :disabled="tuning.saving.value" @click="tuning.discard">{{ tr('discard') }}</MButton>
        <MButton :loading="tuning.saving.value" :disabled="tuning.saving.value || tuning.dirtyCount.value === 0" @click="tuning.save">{{ tuning.saving.value ? tr('saving') : tr('save') }}</MButton>
      </div>
    </Transition>

    <AdvancedWarningDialog :show="warning.open" :scope="warning.scope" @cancel="warning.open = false" @confirm="confirmWarning" />
    <MDialog v-model="restoreOpen" :title="tr('restore_defaults')" icon="restart_alt" tone="info">
      <p>{{ tr('restore_confirm') }}</p>
      <template #actions>
        <MButton variant="text" @click="restoreOpen = false">{{ t.common.cancel }}</MButton>
        <MButton @click="restoreDefaults">{{ tr('restore_defaults') }}</MButton>
      </template>
    </MDialog>
  </MPage>
</template>

<script setup>
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { MPage, MGroup, MSection, MItem, MChoiceItem, MSeg, MButton, MIconButton, MNotice, MFooter, MOverflow, MSwitchItem, MField, MEmpty, MSkeleton, MDialog, MIcon, useSubpage } from 'foundation-ui/vue'
const CARD_ICONS = { haptics: 'mobile_vibrate', vibration: 'mobile_vibrate', lighting: 'lightbulb', light: 'lightbulb', triggers: 'touch_app', trigger: 'touch_app', motion: 'sensors', audio: 'volume_up', sticks: 'gamepad', touchpad: 'touch_app' }
import AdvancedWarningDialog from '../dualsense/AdvancedWarningDialog.vue'
import AdvancedControllerOptions from '../controllersHub/AdvancedControllerOptions.vue'
import TuningRow from '../dualsense/TuningRow.vue'
import TuningColor from '../dualsense/TuningColor.vue'
import TuningTrigger from '../dualsense/TuningTrigger.vue'
import { useGamepadType } from '../../composables/useGamepadType.js'
import { useDualSenseStatus } from '../../composables/useDualSenseStatus.js'
import { useDualSenseTuning } from '../../composables/useDualSenseTuning.js'
import { useDualSenseText } from '../../composables/useDualSenseText.js'
import { statusRows } from '../../composables/dualsenseStatusView.js'
import { PRESET_CARDS } from '../../composables/dualsenseTuning.js'
import { isAdvancedWarningHidden, panelStorage, setAdvancedWarningHidden } from '../../composables/advancedWarning.js'
import { useI18n } from '../../desktop/i18n/index.js'
import { useRouter } from '../../composables/useRouter.js'
import { dualsenseBack, isDualSenseEntry } from '../../composables/gamepadPages.js'

const props = defineProps({ params: { type: Object, default: null } })
const emit = defineEmits(['back', 'open-controller-meta', 'manage-components'])
const { t } = useI18n()
const { tx } = useDualSenseText()
const { current, open, back } = useSubpage('gamepad')
const router = useRouter()
// opened as the DualSense page of the stream settings: the root has a back bubble to 输入
const viaDualsense = computed(() => isDualSenseEntry(router.nav.current.value))
const onBack = () => { if (current.value || !viaDualsense.value) { back(); return } const to = dualsenseBack(); router.navigate(to.route, to.params) }
const gt = useGamepadType()
const tuning = useDualSenseTuning()
const { tr } = tuning
const { status, loading: statusLoading, failed: statusFailed, refresh } = useDualSenseStatus()

const dsVisible = computed(() => gt.hasDs5Selection.value)
const typeOptions = computed(() => gt.modeOptions.value.map((o) => ({ value: o.value, label: o.label, text: o.hint })))
const presetOptions = computed(() => PRESET_CARDS.map((p) => ({ value: p.name, label: tr('presets.' + p.name), icon: { default: 'tune', competitive: 'bolt', immersive: 'graphic_eq', legacy: 'history' }[p.name] || 'tune' })))
const statusLabel = computed(() => (status.value ? tx('status.states.' + status.value.state) : ''))
const statusRowsView = computed(() => statusRows(status.value, (path, params) => tx('status.values.' + path, params)))
const needsTransport = computed(() => ['usbip_missing', 'usbip_version', 'usbip_unsigned', 'usbip_not_ready'].includes(status.value?.state))
const installSteps = computed(() => t.value.dualSense.status.install.steps)
const nextText = computed(() => (status.value ? tx('status.next.' + status.value.state, { pinned: status.value.transport?.pinned_version || '', version: status.value.transport?.version || '' }) : ''))
const cardObj = computed(() => tuning.cards.value.find((c) => 'card/' + c.id === current.value))
const cardRows = computed(() => cardObj.value?.rows || [])
const advGroup = computed(() => tuning.groups.value.find((g) => 'advanced/' + g.id === current.value))

const title = computed(() => {
  const c = current.value
  if (!c) return t.value.m3e.input.gamepad
  if (c === 'per-game') return t.value.controllersHub.emulation.perAppTitle
  if (c === 'status') return 'DualSense'
  if (c.startsWith('card/')) return tr('cards.' + c.slice(5))
  if (c === 'pending') return tr('pending.title')
  if (c === 'advanced') return tr('advanced')
  if (c.startsWith('advanced/')) return tr('groups.' + c.slice(9))
  if (c === 'emulation') return t.value.deviceHub.controllerAdvanced
  return ''
})
// With the supporting pane beside the page the state is shown there; the row would repeat it.
const refreshAll = () => Promise.all([refresh({ force: true }), tuning.load()])

// Both advanced entries ask first (five seconds, "do not show again") unless the warning was switched off.
const warning = reactive({ open: false, scope: 'tuning', target: '' })
const askAdvanced = (target, scope = 'tuning') => {
  if (isAdvancedWarningHidden(panelStorage())) return open(target)
  Object.assign(warning, { open: true, scope, target })
}
const confirmWarning = (dontShowAgain) => {
  if (dontShowAgain) setAdvancedWarningHidden(panelStorage(), true)
  warning.open = false
  open(warning.target)
}
// A page that was open when the panel reloaded is only restored if it is not behind the warning.
// Where a search hit or a recent row lands: the card or the advanced group of the key (an advanced group only behind its warning), then the row
const landOnKey = (focus, tries = 12) => {
  setTimeout(() => {
    const row = document.querySelector(`[data-ds5-key="${CSS.escape(String(focus))}"]`)
    if (!row) { if (tries > 0) landOnKey(focus, tries - 1); return }
    row.scrollIntoView({ block: 'center', behavior: 'smooth' })
    row.classList.add('ds5-hit')
    setTimeout(() => row.classList.remove('ds5-hit'), 2200)
  }, 200)
}
const landOnParams = () => {
  const target = props.params
  if (!target?.sub) return
  if (target.sub.startsWith('g')) askAdvanced('advanced/' + target.sub.slice(1))
  else open('card/' + target.sub)
  if (target.focus) landOnKey(target.focus)
}
onMounted(async () => {
  if (['advanced', 'emulation'].some((p) => current.value.startsWith(p)) && !isAdvancedWarningHidden(panelStorage())) current.value = ''
  await tuning.load()
  landOnParams()
})
watch(() => status.value?.state, (state, previous) => {
  if (previous && state && !tuning.snapshot.value && !['host_unreachable', 'host_unsupported'].includes(state)) tuning.load()
})

const restoreOpen = ref(false)
const restoreDefaults = () => {
  restoreOpen.value = false
  tuning.restoreDefaults()
}
</script>

<style>
.ds5-hit { animation: ds5-hit 2.2s ease-out; }
@keyframes ds5-hit { 0%, 40% { box-shadow: inset 0 0 0 3px var(--md-sys-color-primary); } 100% { box-shadow: inset 0 0 0 3px transparent; } }
</style>
