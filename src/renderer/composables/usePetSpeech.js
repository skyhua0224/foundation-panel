import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { DEFAULT_CONFIG, STORAGE_KEY } from './aiProviders.js'
import { isApiKeyRequired } from './aiClient.js'
import {
  DEFAULT_JITTER_PERCENT, DEFAULT_RANDOM_INTERVAL_SEC, DEFAULT_VISION_INTERVAL_SEC, MAX_INTERVAL_SEC, MAX_JITTER_PERCENT, MIN_INTERVAL_SEC,
  PET_MASTER_KEY, PET_RANDOM_ENABLED_KEY, PET_RANDOM_INTERVAL_KEY, PET_RANDOM_JITTER_KEY, PET_VISION_ENABLED_KEY, PET_VISION_HISTORY_KEY, PET_VISION_INTERVAL_KEY,
  clearVisionHistory, loadJitterPercent, loadMasterEnabled, loadRandomEnabled, loadRandomIntervalSec, loadVisionHistory, notifyPetConfigChanged,
} from './petSpeechConfig.js'

// The speech of the desktop pet: the master switch, the random talk with its interval and its jitter, the toolbar shortcut, the history of what the pet saw (view, clear) and the defaults.
// What the old pet window (PetSettingsTool) offered, in one place for the sheet of 面板设置 › 桌宠; the keys and the notice to the pet are the same.
export const INTERVAL_PRESETS = Object.freeze([15, 30, 60, 120, 300])
export const JITTER_PRESETS = Object.freeze([0, 15, 30, 50])
export const VISION_INTERVAL_PRESETS = Object.freeze([30, 60, 120, 300, 600])

const hasUsableKey = (key) => typeof key === 'string' && Boolean(key.trim()) && !key.includes('****')

const persist = (key, value) => { try { localStorage.setItem(key, value) } catch (error) { console.warn(`[桌宠设置] 配置持久化失败 (${key}):`, error) } }
const clamp = (n, lo, hi) => { const v = Math.round(Number(n)); return Number.isFinite(v) ? Math.min(hi, Math.max(lo, v)) : lo }

export function usePetSpeech() {
  const masterEnabled = ref(loadMasterEnabled())
  const randomEnabled = ref(loadRandomEnabled())
  const randomIntervalSec = ref(loadRandomIntervalSec())
  const jitterPercent = ref(loadJitterPercent())
  const history = ref(loadVisionHistory())
  const historyOpen = ref(false)
  const shortcut = ref({ enabled: true, registered: false, pending: true, error: false })
  let quiet = false
  // the desktop observation needs a configured assistant (the server's configuration, or the one this window kept): without it the switch is off and says why
  const aiReady = ref(false)
  const serverAi = ref(null)
  async function refreshAi() {
    try {
      const proxy = await invoke('get_proxy_url_command')
      const response = await fetch(`${proxy}/api/ai/config`)
      if (!response.ok) throw new Error(String(response.status))
      serverAi.value = { ...DEFAULT_CONFIG, ...(await response.json()), apiKey: '' }
    } catch { serverAi.value = null }
    try {
      const saved = localStorage.getItem(STORAGE_KEY)
      const cfg = serverAi.value || (saved ? { ...DEFAULT_CONFIG, ...JSON.parse(saved) } : { ...DEFAULT_CONFIG })
      aiReady.value = !!(cfg.enabled && (hasUsableKey(cfg.apiKey) || cfg.apiKeyConfigured || !isApiKeyRequired(cfg)))
    } catch { aiReady.value = false }
  }

  const write = (key, value) => { if (!quiet) { persist(key, value); notifyPetConfigChanged() } }
  watch(masterEnabled, (v) => write(PET_MASTER_KEY, String(!!v)))
  watch(randomEnabled, (v) => write(PET_RANDOM_ENABLED_KEY, String(!!v)))
  watch(randomIntervalSec, (v) => { const c = clamp(v, MIN_INTERVAL_SEC, MAX_INTERVAL_SEC); if (c !== v) randomIntervalSec.value = c; else write(PET_RANDOM_INTERVAL_KEY, String(c)) })
  watch(jitterPercent, (v) => { const c = clamp(v, 0, MAX_JITTER_PERCENT); if (c !== v) jitterPercent.value = c; else write(PET_RANDOM_JITTER_KEY, String(c)) })

  const toggleHistory = () => { if (!historyOpen.value) history.value = loadVisionHistory(); historyOpen.value = !historyOpen.value }
  const clearHistory = () => { clearVisionHistory(); history.value = [] }
  const resetDefaults = () => {
    quiet = true
    masterEnabled.value = true
    randomEnabled.value = true
    randomIntervalSec.value = DEFAULT_RANDOM_INTERVAL_SEC
    jitterPercent.value = DEFAULT_JITTER_PERCENT
    persist(PET_MASTER_KEY, 'true'); persist(PET_RANDOM_ENABLED_KEY, 'true'); persist(PET_RANDOM_INTERVAL_KEY, String(DEFAULT_RANDOM_INTERVAL_SEC)); persist(PET_RANDOM_JITTER_KEY, String(DEFAULT_JITTER_PERCENT))
    persist(PET_VISION_ENABLED_KEY, 'false'); persist(PET_VISION_INTERVAL_KEY, String(DEFAULT_VISION_INTERVAL_SEC * 1000))
    Promise.resolve().then(() => { quiet = false })
    notifyPetConfigChanged()
  }

  async function loadShortcut() {
    shortcut.value = { ...shortcut.value, pending: true, error: false }
    try { const s = await invoke('load_toolbar_shortcut_status'); shortcut.value = { enabled: !!s.enabled, registered: !!s.registered, pending: false, error: false } } catch { shortcut.value = { ...shortcut.value, pending: false, error: true } }
  }
  async function setShortcut(enabled) {
    shortcut.value = { ...shortcut.value, pending: true, error: false }
    try { const s = await invoke('set_toolbar_shortcut_enabled', { enabled }); shortcut.value = { enabled: !!s.enabled, registered: !!s.registered, pending: false, error: false } } catch { shortcut.value = { ...shortcut.value, pending: false, error: true } }
  }
  const onStorage = (event) => { if (event.key === PET_VISION_HISTORY_KEY && historyOpen.value) history.value = loadVisionHistory() }
  onMounted(() => { void loadShortcut(); void refreshAi(); window.addEventListener('storage', onStorage) })
  onBeforeUnmount(() => window.removeEventListener('storage', onStorage))

  return { aiReady, refreshAi, masterEnabled, randomEnabled, randomIntervalSec, jitterPercent, history, historyOpen, shortcut, toggleHistory, clearHistory, resetDefaults, setShortcut }
}
