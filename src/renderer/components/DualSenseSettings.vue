<template>
  <div class="ds5t m-ds5t" aria-live="polite">
    <DualSenseStatusCard
      :status="status"
      :loading="statusLoading"
      :failed="statusFailed"
      @refresh="refreshAll"
      @manage-components="emit('manage-components')"
    />

    <MSkeleton v-if="tuning.loading.value && !tuning.snapshot.value" :rows="3" />

    <MNotice v-else-if="tuning.loadError.value && !tuning.unsupported.value" tone="error" :title="tr('load_failed')">
      {{ tuning.loadError.value }}
      <template #action><MButton size="sm" @click="tuning.load">{{ tr('retry') }}</MButton></template>
    </MNotice>

    <template v-else-if="tuning.snapshot.value">
      <MSection :title="tr('title')" :description="tr('wired', { wired: tuning.implementedTotal.value, pending: tuning.keyTotal.value - tuning.implementedTotal.value })">
        <template #action>
          <div class="m-section__actions">
            <MButton variant="text" size="sm" icon="stethoscope" @click="emit('open-controller-meta')">{{ tr('open_meta') }}</MButton>
            <MButton variant="text" size="sm" icon="restart_alt" @click="tuning.restoreDefaults">{{ tr('restore_defaults') }}</MButton>
          </div>
        </template>
        <MNotice v-if="tuning.warning.value" tone="warn">{{ tuning.warning.value }}</MNotice>
        <p v-if="tuning.message.value" class="m-section__desc" role="status">{{ tuning.message.value }}</p>
      </MSection>

      <MSection :title="tr('presets.title')" :description="tuning.selectedPreset.value === 'custom' ? tr('presets.custom') : ''">
        <MGroup role="radiogroup" :aria-label="tr('presets.title')">
          <MItem
            v-for="preset in PRESET_CARDS"
            :key="preset.name"
            action
            :selected="tuning.selectedPreset.value === preset.name"
            :icon="PRESET_ICON[preset.name] || 'tune'"
            :title="tr('presets.' + preset.name)"
            :text="tr('presets.' + preset.name + '_hint')"
            role="radio"
            @click="tuning.applyPreset(preset.name)"
          />
        </MGroup>
      </MSection>

      <MSection v-for="card in tuning.cards.value" :key="card.id" :title="tr('cards.' + card.id)">
        <MGroup>
          <TuningRow
            v-for="key in card.rows"
            :key="key.id"
            :descriptor="key"
            :model-value="tuning.edited[key.id]"
            @update:model-value="tuning.onValue(key.id, $event)"
            @reset="tuning.resetKey(key.id)"
          />
          <TuningColor
            v-if="card.lighting && tuning.hasColor.value"
            :r="Number(tuning.edited[82])"
            :g="Number(tuning.edited[83])"
            :b="Number(tuning.edited[84])"
            :mode="String(tuning.edited[81])"
            @update="tuning.setColor"
          />
        </MGroup>
      </MSection>

      <MSection v-if="tuning.pendingRows.value.length" :title="tr('pending.title')" :description="tr('pending.hint')">
        <MGroup>
          <TuningRow v-for="key in tuning.pendingRows.value" :key="key.id" :descriptor="key" :model-value="tuning.edited[key.id]" disabled />
        </MGroup>
      </MSection>

      <MSection :title="tr('advanced')" :description="tr('advanced_hint', { count: tuning.advancedTotal.value })">
        <MGroup>
          <MItem
            action
            icon="tune"
            :title="tr('advanced')"
            :text="advancedOpen ? tr('collapse') : tr('expand')"
            :aria-expanded="advancedOpen"
            @click="toggleAdvanced"
          >
            <template #trail><MIcon :name="advancedOpen ? 'keyboard_arrow_up' : 'keyboard_arrow_down'" /></template>
          </MItem>
        </MGroup>
        <div v-show="advancedOpen" id="ds5t-advanced-body" class="m-advanced">
          <MField v-model="tuning.search.value" :label="tr('search')" />
          <MGroup>
            <MSwitchItem v-model="tuning.onlyModified.value" :title="tr('only_modified')" />
            <MSwitchItem v-model="tuning.showPending.value" :title="tr('show_pending')" />
          </MGroup>
          <MEmpty v-if="!tuning.groups.value.length" icon="search" :title="tr('no_matches')" />
          <MSection v-for="group in tuning.groups.value" :id="'ds5t-group-' + group.id" :key="group.id" :title="tr('groups.' + group.id)">
            <MGroup>
              <MItem
                action
                :icon="groupIcon(group)"
                :title="tr('groups.' + group.id)"
                :text="tr('group_count', { count: group.wired.length + group.pending.length })"
                :aria-expanded="groupOpen(group.id)"
                @click="toggleGroup(group.id)"
              >
                <template #trail>
                  <MChip v-if="group.wired.length" tone="ok">{{ tr('group_available', { count: group.wired.length }) }}</MChip>
                  <MIcon :name="groupOpen(group.id) ? 'keyboard_arrow_up' : 'keyboard_arrow_down'" />
                </template>
              </MItem>
              <template v-if="groupOpen(group.id)">
                <TuningRow v-for="key in group.wired" :key="key.id" :descriptor="key" :model-value="tuning.edited[key.id]" @update:model-value="tuning.onValue(key.id, $event)" @reset="tuning.resetKey(key.id)" />
                <TuningRow v-for="key in group.pending" :key="key.id" :descriptor="key" :model-value="tuning.edited[key.id]" disabled />
              </template>
            </MGroup>
            <TuningTrigger
              v-if="group.id === 3 && groupOpen(group.id)"
              class="ds5t-preview"
              :enabled="tuning.edited[24] === true"
              :strength="Number(tuning.edited[25] ?? 1)"
              :left-strength="Number(tuning.edited[26] ?? 1)"
              :right-strength="Number(tuning.edited[27] ?? 1)"
            />
          </MSection>
        </div>
      </MSection>
    </template>

    <Transition name="m-savebar">
      <div v-if="tuning.dirtyCount.value > 0 || tuning.saving.value" class="m-savebar" role="region" :aria-label="tr('save')">
        <span class="m-savebar__count">{{ tr('dirty_count', { count: tuning.dirtyCount.value }) }}</span>
        <MButton variant="text" :disabled="tuning.saving.value" @click="tuning.discard">{{ tr('discard') }}</MButton>
        <MButton :loading="tuning.saving.value" :disabled="tuning.saving.value || tuning.dirtyCount.value === 0" @click="tuning.save">{{ tuning.saving.value ? tr('saving') : tr('save') }}</MButton>
      </div>
    </Transition>

    <AdvancedWarningDialog :show="advancedWarningOpen" @cancel="advancedWarningOpen = false" @confirm="confirmAdvancedWarning" />
  </div>
</template>

<script setup>
import { MSection, MGroup, MItem, MSwitchItem, MNotice, MButton, MChip, MIcon, MField, MEmpty, MSkeleton } from 'foundation-ui/vue'
import { onMounted, reactive, ref, watch } from 'vue'
import AdvancedWarningDialog from './dualsense/AdvancedWarningDialog.vue'
import DualSenseStatusCard from './dualsense/DualSenseStatusCard.vue'
import TuningColor from './dualsense/TuningColor.vue'
import TuningRow from './dualsense/TuningRow.vue'
import TuningTrigger from './dualsense/TuningTrigger.vue'
import { PRESET_CARDS } from '../composables/dualsenseTuning.js'
import { isAdvancedWarningHidden, panelStorage, setAdvancedWarningHidden } from '../composables/advancedWarning.js'
import { useDualSenseStatus } from '../composables/useDualSenseStatus.js'
import { useDualSenseTuning } from '../composables/useDualSenseTuning.js'

const emit = defineEmits(['open-controller-meta', 'manage-components'])
const PRESET_ICON = { default: 'restart_alt', competitive: 'bolt', immersive: 'graphic_eq', legacy: 'history' }
const GROUP_ICON = { 1: 'graphic_eq', 2: 'mobile_vibrate', 3: 'touch_app', 4: 'gamepad', 5: 'tune', 6: 'sensors', 7: 'touch_app', 8: 'lightbulb', 9: 'volume_up', 10: 'verified', 11: 'sync', 12: 'swap_horiz' }
const groupIcon = (group) => GROUP_ICON[group.id] || 'tune'

const tuning = useDualSenseTuning()
const { tr } = tuning
const { status, loading: statusLoading, failed: statusFailed, refresh } = useDualSenseStatus()

const ADVANCED_KEY = 'ds5.advanced.open'
const advancedOpen = ref(false)
const advancedWarningOpen = ref(false)
const openGroups = reactive({})
const groupOpen = (id) => tuning.filtering.value || openGroups[id] === true
const toggleGroup = (id) => {
  openGroups[id] = !openGroups[id]
}
const toggleAdvanced = () => {
  if (advancedOpen.value) {
    advancedOpen.value = false
    return
  }
  if (isAdvancedWarningHidden(panelStorage())) {
    advancedOpen.value = true
    return
  }
  advancedWarningOpen.value = true
}
const confirmAdvancedWarning = (dontShowAgain) => {
  if (dontShowAgain) setAdvancedWarningHidden(panelStorage(), true)
  advancedWarningOpen.value = false
  advancedOpen.value = true
}

watch(advancedOpen, (value) => {
  try {
    localStorage.setItem(ADVANCED_KEY, value ? '1' : '0')
  } catch {
    // Storage can be unavailable; the page still works.
  }
})

async function refreshAll() {
  await Promise.all([refresh({ force: true }), tuning.load()])
}

// A host that came back (or was updated) while the page is open gets its table loaded.
watch(() => status.value?.state, (state, previous) => {
  if (previous && state && !tuning.snapshot.value && !['host_unreachable', 'host_unsupported'].includes(state)) {
    tuning.load()
  }
})

onMounted(() => {
  try {
    // A section left open is only restored once the warning has been switched off,
    // so a reload never skips it.
    advancedOpen.value = localStorage.getItem(ADVANCED_KEY) === '1' && isAdvancedWarningHidden(localStorage)
  } catch {
    advancedOpen.value = false
  }
  tuning.load()
})
</script>
