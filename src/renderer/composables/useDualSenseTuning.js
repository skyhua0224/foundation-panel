import { computed, reactive, ref, shallowRef } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { dualsense } from '../tauri-adapter.js'
import {
  advancedCount, advancedGroups, changedValues, colorWired, defaultEdits, detectPreset,
  FIRST_FIXED_COLOR, indexKeys, pendingCommonRows, presetEdits, visibleCards,
} from './dualsenseTuning.js'
import { dualSenseErrorCode, dualSenseErrorText, refusedSaveText } from './dualsenseText.js'
import { useDualSenseText } from './useDualSenseText.js'
import { localizedKeyLabel } from './dualsenseLabels.js'

/**
 * State of the tuning page: the host's table, the person's unsaved edits, and
 * the save round trip. Failures surface as one plain sentence (`loadError`,
 * `message`); `unsupported` is set when the host has no tuning API at all.
 */
export function useDualSenseTuning() {
  const { text, tx, tr, locale } = useDualSenseText()
  const snapshot = shallowRef(null)
  const edited = reactive({})
  const dirty = reactive({})
  const loading = ref(true)
  const loadError = ref('')
  const unsupported = ref(false)
  const saving = ref(false)
  const message = ref('')
  const selectedPreset = ref('custom')
  const search = ref('')
  const onlyModified = ref(false)
  const showPending = ref(true)

  const index = computed(() => indexKeys(snapshot.value))
  const dirtyCount = computed(() => Object.keys(dirty).filter((id) => dirty[id]).length)
  const cards = computed(() => visibleCards(index.value))
  const pendingRows = computed(() => pendingCommonRows(index.value))
  const hasColor = computed(() => colorWired(index.value))
  const groups = computed(() => advancedGroups(snapshot.value, {
    search: search.value,
    onlyModified: onlyModified.value,
    showPending: showPending.value,
    dirty,
    locale: locale.value,
  }))
  const filtering = computed(() => search.value.trim() !== '' || onlyModified.value)
  const advancedTotal = computed(() => advancedCount(snapshot.value))
  const implementedTotal = computed(() => Number(snapshot.value?.implemented_count || 0))
  const keyTotal = computed(() => snapshot.value?.keys?.length || 0)
  const warning = computed(() => {
    const body = snapshot.value
    if (!body) return ''
    // The host's own words are English diagnostics; say it plainly instead.
    return body.load_error || body.load_warnings?.length ? tr('profile_warning') : ''
  })

  function setValue(id, value) {
    edited[id] = value
    dirty[id] = true
    if (selectedPreset.value !== 'custom') selectedPreset.value = 'custom'
    message.value = ''
  }

  function setColor({ r, g, b }) {
    setValue(82, r)
    setValue(83, g)
    setValue(84, b)
  }

  function onValue(id, value) {
    const blackColor = ![82, 83, 84].some((key) => Number(edited[key]))
    if (id === 81 && value === 'fixed' && blackColor) setColor(FIRST_FIXED_COLOR)
    setValue(id, value)
  }

  function resetKey(id) {
    const key = index.value.byId.get(id)
    if (key?.implemented) setValue(id, key.default)
  }

  function applyEdits(edits) {
    for (const [id, value] of Object.entries(edits)) {
      edited[id] = value
      dirty[id] = true
    }
  }

  function applyPreset(name) {
    applyEdits(presetEdits(index.value, name))
    selectedPreset.value = name
    message.value = tr('presets.applied', { name: tr('presets.' + name) })
  }

  function restoreDefaults() {
    applyEdits(defaultEdits(snapshot.value, edited))
    selectedPreset.value = detectPreset(index.value, edited)
    message.value = tr('defaults_restored')
  }

  function discard() {
    for (const key of snapshot.value?.keys || []) {
      edited[key.id] = key.value
      delete dirty[key.id]
    }
    selectedPreset.value = detectPreset(index.value, edited)
    message.value = ''
  }

  function hydrate(body) {
    snapshot.value = body
    for (const key of body.keys || []) {
      edited[key.id] = key.value
      delete dirty[key.id]
    }
    selectedPreset.value = detectPreset(indexKeys(body), edited)
  }

  async function load() {
    loading.value = true
    loadError.value = ''
    unsupported.value = false
    const result = await dualsense.getTuning()
    if (result.success) {
      hydrate(result.data)
    } else {
      unsupported.value = dualSenseErrorCode(result.message) === 'DS5-HOST-002'
      loadError.value = dualSenseErrorText(result.message, text.value.errors, 'loadFallback')
    }
    loading.value = false
  }

  async function confirmExpert(keys) {
    if (!keys.length) return true
    const names = keys.map((key) => localizedKeyLabel(key, locale.value)).join('、')
    try {
      await ElMessageBox.confirm(tr('expert_confirm', { keys: names }), tr('expert'), {
        type: 'warning',
        confirmButtonText: tr('save'),
        cancelButtonText: tr('discard'),
      })
      return true
    } catch {
      return false
    }
  }

  async function save() {
    const { values, expert } = changedValues(snapshot.value, edited, dirty)
    if (!Object.keys(values).length) {
      message.value = tr('nothing_dirty')
      return
    }
    if (!(await confirmExpert(expert))) return
    saving.value = true
    message.value = ''
    const result = await dualsense.setTuning(values)
    saving.value = false
    if (!result.success) {
      message.value = dualSenseErrorText(result.message, text.value.errors, 'saveFallback')
      ElMessage.error(message.value)
      return
    }
    const answer = result.data
    if (answer.snapshot) hydrate(answer.snapshot)
    if (!answer.accepted) {
      message.value = refusedSaveText(answer.code, text.value.refused)
      ElMessage.error(message.value)
      return
    }
    const applied = answer.applied?.length || 0
    const unavailable = answer.unavailable?.length || 0
    message.value = unavailable
      ? tr('save_partial', { applied, unavailable })
      : tr('save_ok', { count: applied })
    ElMessage.success(message.value)
  }

  return {
    snapshot, edited, dirty, loading, loadError, unsupported, saving, message, selectedPreset,
    search, onlyModified, showPending, dirtyCount, cards, pendingRows, hasColor, groups,
    filtering, advancedTotal, implementedTotal, keyTotal, warning,
    load, save, discard, onValue, setColor, resetKey, applyPreset, restoreDefaults, tx, tr,
  }
}
