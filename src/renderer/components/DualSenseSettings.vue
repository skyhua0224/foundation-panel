<!--
  DualSense page of the Controllers pane. The host builds the DualSense path in,
  so this page installs nothing: it shows the real status (DualSenseStatusCard)
  and drives the host's tuning profile (ALK-DS5T/1). Layout follows the Web UI
  tuning page: presets, common settings in two-column cards, advanced settings
  collapsed into function groups, settings the running host cannot apply listed
  apart. A host without /api/dualsense/tuning only gets the status card.
-->
<template>
  <section class="ds5t" aria-live="polite">
    <DualSenseStatusCard
      :status="status"
      :loading="statusLoading"
      :failed="statusFailed"
      @refresh="refreshAll"
      @manage-components="emit('manage-components')"
    />

    <div v-if="tuning.loading.value && !tuning.snapshot.value" class="ds5t-grid" aria-busy="true">
      <div class="ds5t-skeleton"></div>
      <div class="ds5t-skeleton"></div>
    </div>

    <div v-else-if="tuning.loadError.value && !tuning.unsupported.value" class="ds5t-alert ds5t-surface" role="alert">
      <strong>{{ tr('load_failed') }}</strong>
      <span>{{ tuning.loadError.value }}</span>
      <button type="button" class="ds5t-btn" @click="tuning.load">{{ tr('retry') }}</button>
    </div>

    <template v-else-if="tuning.snapshot.value">
      <header class="ds5t-surface ds5t-head">
        <div class="ds5t-head-main">
          <h2 class="ds5t-title">{{ tr('title') }}</h2>
          <p class="ds5t-meta">
            <span>{{ tr('wired', { wired: tuning.implementedTotal.value, total: tuning.keyTotal.value }) }}</span>
            <span v-if="tuning.message.value" class="ds5t-meta-note" role="status">{{ tuning.message.value }}</span>
          </p>
        </div>
        <button type="button" class="ds5t-btn ds5t-btn-ghost" @click="emit('open-controller-meta')">
          <el-icon><Aim /></el-icon>{{ tr('open_meta') }}
        </button>
        <button type="button" class="ds5t-btn ds5t-btn-ghost" @click="tuning.restoreDefaults">
          <el-icon><RefreshLeft /></el-icon>{{ tr('restore_defaults') }}
        </button>
      </header>

      <p v-if="tuning.warning.value" class="ds5t-alert is-warning ds5t-surface" role="status">{{ tuning.warning.value }}</p>

      <section class="ds5t-surface ds5t-presets-card" aria-labelledby="ds5t-presets-title">
        <div class="ds5t-card-head">
          <span class="ds5t-chip-icon"><el-icon><MagicStick /></el-icon></span>
          <h3 id="ds5t-presets-title" class="ds5t-card-title">{{ tr('presets.title') }}</h3>
          <span v-if="tuning.selectedPreset.value === 'custom'" class="ds5t-card-hint">{{ tr('presets.custom') }}</span>
        </div>
        <div class="ds5t-presets" role="radiogroup" :aria-label="tr('presets.title')">
          <button
            v-for="preset in PRESET_CARDS"
            :key="preset.name"
            type="button"
            class="ds5t-preset"
            :class="{ selected: tuning.selectedPreset.value === preset.name }"
            role="radio"
            :aria-checked="tuning.selectedPreset.value === preset.name"
            @click="tuning.applyPreset(preset.name)"
          >
            <el-icon><component :is="icons[preset.icon]" /></el-icon>
            <span class="ds5t-preset-text">
              <span class="ds5t-preset-name">{{ tr('presets.' + preset.name) }}</span>
              <span class="ds5t-preset-hint">{{ tr('presets.' + preset.name + '_hint') }}</span>
            </span>
          </button>
        </div>
      </section>

      <div class="ds5t-grid">
        <section
          v-for="card in tuning.cards.value"
          :key="card.id"
          class="ds5t-surface ds5t-card"
          :aria-labelledby="'ds5t-card-' + card.id"
        >
          <div class="ds5t-card-head">
            <span class="ds5t-chip-icon"><el-icon><component :is="icons[card.icon]" /></el-icon></span>
            <h3 :id="'ds5t-card-' + card.id" class="ds5t-card-title">{{ tr('cards.' + card.id) }}</h3>
          </div>
          <div class="ds5t-rows">
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
          </div>
        </section>
      </div>

      <section
        v-if="tuning.pendingRows.value.length"
        class="ds5t-surface ds5t-card is-quiet"
        aria-labelledby="ds5t-pending-title"
      >
        <div class="ds5t-card-head">
          <span class="ds5t-chip-icon"><el-icon><Clock /></el-icon></span>
          <h3 id="ds5t-pending-title" class="ds5t-card-title">{{ tr('pending.title') }}</h3>
          <span class="ds5t-card-hint">{{ tr('pending.hint') }}</span>
        </div>
        <div class="ds5t-rows ds5t-rows-wide">
          <TuningRow
            v-for="key in tuning.pendingRows.value"
            :key="key.id"
            :descriptor="key"
            :model-value="tuning.edited[key.id]"
            disabled
          />
        </div>
      </section>

      <section class="ds5t-surface ds5t-card" aria-labelledby="ds5t-advanced-title">
        <div class="ds5t-card-head">
          <span class="ds5t-chip-icon"><el-icon><Operation /></el-icon></span>
          <h3 id="ds5t-advanced-title" class="ds5t-card-title">{{ tr('advanced') }}</h3>
          <span class="ds5t-card-hint">{{ tr('advanced_hint', { count: tuning.advancedTotal.value }) }}</span>
          <button
            type="button"
            class="ds5t-btn ds5t-btn-ghost ds5t-btn-sm ds5t-head-action"
            :aria-expanded="advancedOpen"
            aria-controls="ds5t-advanced-body"
            @click="toggleAdvanced"
          >
            {{ advancedOpen ? tr('collapse') : tr('expand') }}
            <el-icon><component :is="advancedOpen ? icons.ArrowUp : icons.ArrowDown" /></el-icon>
          </button>
        </div>

        <div v-show="advancedOpen" id="ds5t-advanced-body" class="ds5t-advanced-body">
          <div class="ds5t-toolbar">
            <label class="ds5t-search">
              <el-icon><Search /></el-icon>
              <input v-model="tuning.search.value" type="search" :placeholder="tr('search')" :aria-label="tr('search')" />
            </label>
            <label class="ds5t-check">
              <input v-model="tuning.onlyModified.value" class="ds5t-switch-mini" type="checkbox" role="switch" />
              <span>{{ tr('only_modified') }}</span>
            </label>
            <label class="ds5t-check">
              <input v-model="tuning.showPending.value" class="ds5t-switch-mini" type="checkbox" role="switch" />
              <span>{{ tr('show_pending') }}</span>
            </label>
          </div>

          <div v-if="!tuning.groups.value.length" class="ds5t-empty">{{ tr('no_matches') }}</div>

          <div v-else class="ds5t-group-grid">
            <div
              v-for="group in tuning.groups.value"
              :key="group.id"
              class="ds5t-group"
              :class="{ 'is-open': groupOpen(group.id) }"
            >
              <button
                type="button"
                class="ds5t-group-head"
                :aria-expanded="groupOpen(group.id)"
                :aria-controls="'ds5t-group-body-' + group.id"
                @click="toggleGroup(group.id)"
              >
                <span class="ds5t-chip-icon is-small"><el-icon><component :is="icons[group.icon]" /></el-icon></span>
                <span class="ds5t-group-title">{{ tr('groups.' + group.id) }}</span>
                <span class="ds5t-group-meta">
                  <span v-if="group.wired.length" class="ds5t-group-live">{{ tr('group_available', { count: group.wired.length }) }}</span>
                  <span>{{ tr('group_count', { count: group.wired.length + group.pending.length }) }}</span>
                </span>
                <el-icon><component :is="groupOpen(group.id) ? icons.ArrowUp : icons.ArrowDown" /></el-icon>
              </button>
              <div v-if="groupOpen(group.id)" :id="'ds5t-group-body-' + group.id" class="ds5t-group-body">
                <div class="ds5t-rows ds5t-rows-wide">
                  <TuningRow
                    v-for="key in group.wired"
                    :key="key.id"
                    :descriptor="key"
                    :model-value="tuning.edited[key.id]"
                    @update:model-value="tuning.onValue(key.id, $event)"
                    @reset="tuning.resetKey(key.id)"
                  />
                  <p v-if="group.wired.length && group.pending.length" class="ds5t-caption">{{ tr('pending.title') }}</p>
                  <TuningRow
                    v-for="key in group.pending"
                    :key="key.id"
                    :descriptor="key"
                    :model-value="tuning.edited[key.id]"
                    disabled
                  />
                </div>
                <TuningTrigger
                  v-if="group.id === 3"
                  class="ds5t-preview"
                  :enabled="tuning.edited[24] === true"
                  :strength="Number(tuning.edited[25] ?? 1)"
                  :left-strength="Number(tuning.edited[26] ?? 1)"
                  :right-strength="Number(tuning.edited[27] ?? 1)"
                />
              </div>
            </div>
          </div>
        </div>
      </section>
    </template>

    <AdvancedWarningDialog
      :show="advancedWarningOpen"
      @cancel="advancedWarningOpen = false"
      @confirm="confirmAdvancedWarning"
    />

    <Transition name="ds5t-bar">
      <div v-if="tuning.dirtyCount.value > 0 || tuning.saving.value" class="ds5t-savebar" role="region" :aria-label="tr('save')">
        <div class="ds5t-savebar-inner">
          <span class="ds5t-save-count">{{ tr('dirty_count', { count: tuning.dirtyCount.value }) }}</span>
          <div class="ds5t-save-actions">
            <button type="button" class="ds5t-btn ds5t-btn-ghost" :disabled="tuning.saving.value" @click="tuning.discard">
              {{ tr('discard') }}
            </button>
            <button
              type="button"
              class="ds5t-btn ds5t-btn-primary"
              :disabled="tuning.saving.value || tuning.dirtyCount.value === 0"
              @click="tuning.save"
            >
              {{ tuning.saving.value ? tr('saving') : tr('save') }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </section>
</template>

<script setup>
import '../styles/DualSenseTuning.less'
import { onMounted, reactive, ref, watch } from 'vue'
import { Aim, Clock, MagicStick, Operation, RefreshLeft, Search } from '@element-plus/icons-vue'
import AdvancedWarningDialog from './dualsense/AdvancedWarningDialog.vue'
import DualSenseStatusCard from './dualsense/DualSenseStatusCard.vue'
import TuningColor from './dualsense/TuningColor.vue'
import TuningRow from './dualsense/TuningRow.vue'
import TuningTrigger from './dualsense/TuningTrigger.vue'
import { PRESET_CARDS } from '../composables/dualsenseTuning.js'
import { isAdvancedWarningHidden, panelStorage, setAdvancedWarningHidden } from '../composables/advancedWarning.js'
import { DUALSENSE_ICONS as icons } from '../composables/dualsenseIcons.js'
import { useDualSenseStatus } from '../composables/useDualSenseStatus.js'
import { useDualSenseTuning } from '../composables/useDualSenseTuning.js'

const emit = defineEmits(['open-controller-meta', 'manage-components'])

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
