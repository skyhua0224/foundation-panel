<template>
  <div class="m-pet">
    <header class="m-pet__head">
      <h2>{{ t.petTool.title }}</h2>
      <MButton variant="text" icon="language" :title="t.petTool.languageDesc" @click="onToggleLocale">{{ locale === 'zh' ? 'EN' : '中' }}</MButton>
      <MIconButton icon="close" :label="t.toolWindow.close" @click="$emit('close')" />
    </header>
    <MSeg v-model="activeTab" block :label="t.petTool.title" :options="tabOptions" />

    <section class="m-pet__panel" @touchstart.passive="onTabTouchStart" @touchend.passive="onTabTouchEnd" @touchcancel="resetTabTouch">
      <!-- 对话 -->
      <div v-if="activeTab === 'speech'" class="m-pet__content">
        <MGroup>
          <MSwitchItem v-model="masterEnabled" icon="forum" :title="t.petTool.master" :text="t.petTool.masterDesc" />
        </MGroup>

        <MSection>
          <MGroup>
            <MSwitchItem v-model="randomEnabled" icon="forum" :title="t.petTool.random" :text="t.petTool.randomDesc" :disabled="!masterEnabled" />
            <template v-if="masterEnabled && randomEnabled">
              <MItem stack icon="timer" :title="t.petTool.speechInterval" :text="t.petTool.speechIntervalDesc">
                <template #control>
                  <div class="m-pet__set">
                    <MChipSet v-model="randomIntervalSec" :label="t.petTool.speechInterval" :options="INTERVAL_PRESETS.map((v) => ({ value: v, label: formatSec(v) }))" />
                    <MField v-model.number="randomIntervalSec" type="number" :label="t.petTool.unitSec" :error="outside(randomIntervalSec, MIN_INTERVAL_SEC, MAX_INTERVAL_SEC) ? rangeText(MIN_INTERVAL_SEC, MAX_INTERVAL_SEC) : ''" />
                  </div>
                </template>
              </MItem>
              <MItem stack icon="tune" :title="t.petTool.jitter" :text="t.petTool.jitterDesc">
                <template #control>
                  <div class="m-pet__set">
                    <MChipSet v-model="jitterPercent" :label="t.petTool.jitter" :options="JITTER_PRESETS.map((v) => ({ value: v, label: `${v}%` }))" />
                    <MField v-model.number="jitterPercent" type="number" label="%" :error="outside(jitterPercent, 0, MAX_JITTER_PERCENT) ? rangeText(0, MAX_JITTER_PERCENT) : ''" />
                  </div>
                </template>
              </MItem>
            </template>
          </MGroup>
        </MSection>

        <MSection>
          <MBanner v-if="masterEnabled && !aiConfigReady" tone="warn" icon="key">{{ t.petTool.aiKeyMissing }}. {{ t.petTool.aiKeyMissingDesc }}</MBanner>
          <MGroup>
            <MSwitchItem :model-value="visionEnabled" icon="visibility" :title="t.petTool.vision" :text="t.petTool.visionDesc" :info="t.petTool.visionHelp" :disabled="visionToggleDisabled || visionConfirmPending" @update:model-value="onVisionToggle" />
            <template v-if="masterEnabled && visionEnabled">
              <MItem stack icon="schedule" :title="t.petTool.visionInterval" :text="t.petTool.visionIntervalDesc">
                <template #control>
                  <div class="m-pet__set">
                    <MChipSet v-model="visionIntervalSec" :label="t.petTool.visionInterval" :options="VISION_INTERVAL_PRESETS.map((v) => ({ value: v, label: formatSec(v) }))" />
                    <MField v-model.number="visionIntervalSec" type="number" :label="t.petTool.unitSec" :error="outside(visionIntervalSec, MIN_INTERVAL_SEC, MAX_INTERVAL_SEC) ? rangeText(MIN_INTERVAL_SEC, MAX_INTERVAL_SEC) : ''" />
                    <MNotice v-if="visionIntervalSec < 60" tone="warn">{{ t.petTool.visionCostHint }}</MNotice>
                  </div>
                </template>
              </MItem>
              <MItem action icon="bolt" :title="t.petTool.pokeNow" :text="pokeFailed ? t.petTool.pokeFailed : t.petTool.pokeNowDesc" :disabled="pokePending" @click="onPokeNow">
                <template #trail><MLoading v-if="pokePending" size="20px" /><span v-else class="m-pet__act">{{ t.petTool.pokeBtn }}</span></template>
              </MItem>
              <MItem action icon="history" :title="t.petTool.visionHistory" :text="t.petTool.visionHistoryDesc" @click="toggleHistory">
                <template #trail><span class="m-pet__act">{{ t.petTool.visionHistoryView }}</span></template>
              </MItem>
            </template>
          </MGroup>
          <template v-if="masterEnabled && visionEnabled && showHistory">
            <MEmpty v-if="visionHistory.length === 0" icon="history" :title="t.petTool.visionHistoryEmpty" />
            <template v-else>
              <MGroup><MItem v-for="(item, idx) in visionHistory" :key="idx" :title="item.text" :value="formatHistoryTime(item.timestamp)" /></MGroup>
              <div class="m-pet__row"><MButton variant="text" icon="delete" @click="onClearHistory">{{ t.petTool.visionHistoryClear }}</MButton></div>
            </template>
          </template>
        </MSection>

        <div class="m-pet__row"><MButton variant="text" icon="restart_alt" :title="t.petTool.resetTooltip" @click="resetSpeechDefaults">{{ t.petTool.reset }}</MButton></div>
      </div>

      <!-- 动画 -->
      <div v-else-if="activeTab === 'animation'" class="m-pet__content"><MEmpty icon="motion_blur" :title="t.petTool.animationMore" /></div>

      <!-- 快捷键 -->
      <div v-else class="m-pet__content">
        <MGroup>
          <MSwitchItem :model-value="toolbarShortcutEnabled" icon="keyboard" :title="t.petTool.toolbarShortcut" :text="t.petTool.toolbarShortcutDesc" :disabled="toolbarShortcutPending" @update:model-value="onToolbarShortcutToggle" />
        </MGroup>
        <MNotice v-if="toolbarShortcutUnavailable" tone="warn">{{ t.petTool.toolbarShortcutUnavailable }}</MNotice>
        <MNotice v-else-if="toolbarShortcutStateError" tone="error">{{ t.petTool.toolbarShortcutStateError }}</MNotice>
      </div>
    </section>

    <MDialog :model-value="visionConfirmOpen" :title="t.petTool.visionPrivacyConfirm.title" icon="visibility" tone="info" @update:model-value="(v) => { if (!v) finishVisionConfirmation(false) }">
      <p>{{ t.petTool.visionPrivacyConfirm.message }}</p>
      <template #actions>
        <MButton variant="text" @click="finishVisionConfirmation(false)">{{ t.petTool.visionPrivacyConfirm.cancel }}</MButton>
        <MButton @click="finishVisionConfirmation(true)">{{ t.petTool.visionPrivacyConfirm.accept }}</MButton>
      </template>
    </MDialog>
  </div>
</template>

<script setup>
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from '../../desktop/i18n/index.js'
import { STORAGE_KEY, DEFAULT_CONFIG } from '../../composables/aiProviders.js'
import { isApiKeyRequired } from '../../composables/aiClient.js'
import { MBanner, MButton, MChipSet, MDialog, MEmpty, MField, MGroup, MIconButton, MItem, MLoading, MNotice, MSection, MSeg, MSwitchItem } from 'foundation-ui/vue'
import {
  PET_MASTER_KEY,
  PET_RANDOM_ENABLED_KEY,
  PET_RANDOM_INTERVAL_KEY,
  PET_RANDOM_JITTER_KEY,
  PET_VISION_ENABLED_KEY,
  PET_VISION_INTERVAL_KEY,
  MIN_INTERVAL_SEC,
  MAX_INTERVAL_SEC,
  MAX_JITTER_PERCENT,
  DEFAULT_RANDOM_INTERVAL_SEC,
  DEFAULT_VISION_INTERVAL_SEC,
  DEFAULT_JITTER_PERCENT,
  loadMasterEnabled,
  loadRandomEnabled,
  loadRandomIntervalSec,
  loadJitterPercent,
  loadVisionEnabled,
  loadVisionIntervalSec,
  PET_VISION_HISTORY_KEY,
  loadVisionHistory,
  clearVisionHistory,
  notifyPetConfigChanged,
  requestPetVision,
} from '../../composables/petSpeechConfig.js'

const { t, locale, toggleLocale } = useI18n()

defineEmits(['close'])

const TABS = Object.freeze([
  { id: 'speech', icon: 'forum' },
  { id: 'animation', icon: 'motion_blur' },
  { id: 'shortcuts', icon: 'keyboard' },
])
const tabOptions = computed(() => TABS.map((tab) => ({ value: tab.id, icon: tab.icon, label: t.value.petTool.tabs[tab.id] })))
// a typed value outside the allowed range says so; the stored value is clamped (the watches below) as before
const outside = (v, min, max) => !(Number(v) >= min && Number(v) <= max)
const rangeText = (min, max) => `${min} – ${max}`

const INTERVAL_PRESETS = Object.freeze([15, 30, 60, 120, 300])
const VISION_INTERVAL_PRESETS = Object.freeze([30, 60, 120, 300, 600])
const JITTER_PRESETS = Object.freeze([0, 15, 30, 50])

const activeTab = ref('speech')
const masterEnabled = ref(loadMasterEnabled())
const randomEnabled = ref(loadRandomEnabled())
const randomIntervalSec = ref(loadRandomIntervalSec())
const jitterPercent = ref(loadJitterPercent())
const visionEnabled = ref(loadVisionEnabled())
const visionIntervalSec = ref(loadVisionIntervalSec())
const aiConfigVersion = ref(0)
const serverAiConfig = ref(null)
const visionConfirmPending = ref(false)
const visionConfirmOpen = ref(false)
const toolbarShortcutEnabled = ref(true)
const toolbarShortcutRegistered = ref(false)
const toolbarShortcutPending = ref(true)
const toolbarShortcutStateError = ref(false)
let tabTouchStart = null
let aiConfigRefreshGeneration = 0

const toolbarShortcutUnavailable = computed(
  () => !toolbarShortcutPending.value
    && !toolbarShortcutStateError.value
    && toolbarShortcutEnabled.value
    && !toolbarShortcutRegistered.value,
)

async function loadToolbarShortcutStatus() {
  toolbarShortcutPending.value = true
  toolbarShortcutStateError.value = false
  try {
    const status = await invoke('load_toolbar_shortcut_status')
    toolbarShortcutEnabled.value = Boolean(status.enabled)
    toolbarShortcutRegistered.value = Boolean(status.registered)
  } catch (error) {
    console.warn('[桌宠设置] 加载快捷键状态失败:', error)
    toolbarShortcutStateError.value = true
  } finally {
    toolbarShortcutPending.value = false
  }
}

async function onToolbarShortcutToggle(enabled) {
  toolbarShortcutPending.value = true
  toolbarShortcutStateError.value = false
  try {
    const status = await invoke('set_toolbar_shortcut_enabled', { enabled })
    toolbarShortcutEnabled.value = Boolean(status.enabled)
    toolbarShortcutRegistered.value = Boolean(status.registered)
  } catch (error) {
    console.warn('[桌宠设置] 保存快捷键状态失败:', error)
    toolbarShortcutStateError.value = true
  } finally {
    toolbarShortcutPending.value = false
  }
}

function hasUsableApiKey(key) {
  return typeof key === 'string' && Boolean(key.trim()) && !key.includes('****')
}

// AI 配置状态（决定桌面观察是否可用）
const aiConfigReady = computed(() => {
  const configVersion = aiConfigVersion.value
  if (!Number.isFinite(configVersion)) return false
  try {
    const saved = localStorage.getItem(STORAGE_KEY)
    const localConfig = saved ? { ...DEFAULT_CONFIG, ...JSON.parse(saved) } : { ...DEFAULT_CONFIG }
    const cfg = serverAiConfig.value || localConfig
    return !!(cfg.enabled && (hasUsableApiKey(cfg.apiKey) || cfg.apiKeyConfigured || !isApiKeyRequired(cfg)))
  } catch {
    return false
  }
})

async function refreshAiConfigStatus() {
  const generation = ++aiConfigRefreshGeneration
  try {
    const proxyUrl = await invoke('get_proxy_url_command')
    const response = await fetch(`${proxyUrl}/api/ai/config`)
    if (!response.ok) throw new Error(`Failed to load AI configuration (${response.status})`)
    const remoteConfig = await response.json()
    if (generation !== aiConfigRefreshGeneration) return false
    serverAiConfig.value = { ...DEFAULT_CONFIG, ...remoteConfig, apiKey: '' }
  } catch {
    if (generation !== aiConfigRefreshGeneration) return false
    serverAiConfig.value = null
  }
  return true
}

function correctVisionEnabled() {
  if (!aiConfigReady.value && visionEnabled.value) visionEnabled.value = false
}

const visionToggleDisabled = computed(() => !masterEnabled.value || !aiConfigReady.value)

// 桌面观察历史
const visionHistory = ref(loadVisionHistory())
const showHistory = ref(false)
const pokePending = ref(false)
const pokeFailed = ref(false)

function refreshHistory() {
  visionHistory.value = loadVisionHistory()
}

function toggleHistory() {
  if (!showHistory.value) refreshHistory()
  showHistory.value = !showHistory.value
}

function onClearHistory() {
  clearVisionHistory()
  visionHistory.value = []
}

function formatHistoryTime(ts) {
  try {
    const d = new Date(ts)
    const hh = String(d.getHours()).padStart(2, '0')
    const mm = String(d.getMinutes()).padStart(2, '0')
    return `${hh}:${mm}`
  } catch {
    return ''
  }
}

function formatSec(sec) {
  if (sec >= 60) return `${Math.round(sec / 60)} ${t.value.petTool.unitMin}`
  return `${sec} ${t.value.petTool.unitSec}`
}

function onToggleLocale() {
  toggleLocale()
}

function onTabTouchStart(event) {
  if (event.touches.length !== 1) return
  if (event.target?.closest?.('button, input, label, a, [role="button"], md-outlined-text-field, md-switch')) return

  const touch = event.touches[0]
  tabTouchStart = {
    x: touch.clientX,
    y: touch.clientY,
    startedAt: performance.now(),
  }
}

function resetTabTouch() {
  tabTouchStart = null
}

function onTabTouchEnd(event) {
  const start = tabTouchStart
  resetTabTouch()
  if (!start || event.changedTouches.length !== 1) return

  const touch = event.changedTouches[0]
  const deltaX = touch.clientX - start.x
  const deltaY = touch.clientY - start.y
  const elapsed = performance.now() - start.startedAt
  if (elapsed > 800 || Math.abs(deltaX) < 56 || Math.abs(deltaX) < Math.abs(deltaY) * 1.2) {
    return
  }

  const currentIndex = TABS.findIndex((tab) => tab.id === activeTab.value)
  const nextIndex = deltaX < 0 ? currentIndex + 1 : currentIndex - 1
  if (nextIndex >= 0 && nextIndex < TABS.length) {
    activeTab.value = TABS[nextIndex].id
  }
}

function emitConfigChanged() {
  notifyPetConfigChanged()
}

function persistSetting(key, value) {
  try {
    localStorage.setItem(key, value)
  } catch (error) {
    console.warn(`[桌宠设置] 配置持久化失败 (${key}):`, error)
  }
}

function clampInterval(n) {
  const v = Math.round(Number(n))
  if (!Number.isFinite(v)) return MIN_INTERVAL_SEC
  return Math.min(MAX_INTERVAL_SEC, Math.max(MIN_INTERVAL_SEC, v))
}

function clampJitter(n) {
  const v = Math.round(Number(n))
  if (!Number.isFinite(v) || v < 0) return 0
  return Math.min(MAX_JITTER_PERCENT, v)
}

// 护栏：避免 watch 被 clamp 后的写回再次触发
let suppressWatch = false
function safeAssign(refObj, value) {
  if (refObj.value === value) return
  suppressWatch = true
  refObj.value = value
  // 下一个微任务释放，仅拦截本次同步触发的 watch
  Promise.resolve().then(() => { suppressWatch = false })
}

// 总开关
watch(masterEnabled, (v) => {
  if (suppressWatch) return
  persistSetting(PET_MASTER_KEY, String(!!v))
  emitConfigChanged()
})

// 随机对话开关
watch(randomEnabled, (v) => {
  if (suppressWatch) return
  persistSetting(PET_RANDOM_ENABLED_KEY, String(!!v))
  emitConfigChanged()
})

// 随机对话间隔：clamp 后写入
watch(randomIntervalSec, (v) => {
  if (suppressWatch) return
  const c = clampInterval(v)
  if (c !== v) {
    safeAssign(randomIntervalSec, c)
  }
  persistSetting(PET_RANDOM_INTERVAL_KEY, String(c))
  emitConfigChanged()
})

// 拖动幅度
watch(jitterPercent, (v) => {
  if (suppressWatch) return
  const c = clampJitter(v)
  if (c !== v) {
    safeAssign(jitterPercent, c)
  }
  persistSetting(PET_RANDOM_JITTER_KEY, String(c))
  emitConfigChanged()
})

// 桌面观察开关：AI Key 未配置时强制下位
watch(visionEnabled, (v) => {
  if (suppressWatch) return
  if (v && !aiConfigReady.value) {
    safeAssign(visionEnabled, false)
    return
  }
  persistSetting(PET_VISION_ENABLED_KEY, String(!!v))
  emitConfigChanged()
})

// 桌面观察间隔：clamp 后以毫秒写入（与 useDesktopPet 兼容）
watch(visionIntervalSec, (v) => {
  if (suppressWatch) return
  const c = clampInterval(v)
  if (c !== v) {
    safeAssign(visionIntervalSec, c)
  }
  persistSetting(PET_VISION_INTERVAL_KEY, String(c * 1000))
  emitConfigChanged()
})

function setRandomIntervalPreset(sec) {
  randomIntervalSec.value = sec
}

function setJitterPreset(pct) {
  jitterPercent.value = pct
}

function setVisionIntervalPreset(sec) {
  visionIntervalSec.value = sec
}

function onVisionToggle(nextValue) {

  if (!nextValue) {
    visionEnabled.value = false
    return
  }
  if (visionToggleDisabled.value || visionConfirmPending.value) return

  visionConfirmPending.value = true
  visionConfirmOpen.value = true
}

function finishVisionConfirmation(accepted) {
  if (!visionConfirmPending.value) return
  visionConfirmOpen.value = false
  visionConfirmPending.value = false
  if (accepted && !visionToggleDisabled.value) visionEnabled.value = true
}

async function onPokeNow() {
  if (!aiConfigReady.value || !visionEnabled.value || pokePending.value) return
  pokePending.value = true
  pokeFailed.value = false
  try {
    const result = await requestPetVision({ ensureToolbar: true })
    if (result?.success) {
      refreshHistory()
    } else {
      pokeFailed.value = true
    }
  } catch (error) {
    console.warn('[桌宠设置] 立即观察失败:', error)
    pokeFailed.value = true
  } finally {
    pokePending.value = false
  }
}

// 恢复对话相关默认（总开关 / 随机对话 / 桌面观察 / 各间隔）
function resetSpeechDefaults() {
  suppressWatch = true
  masterEnabled.value = true
  randomEnabled.value = true
  randomIntervalSec.value = DEFAULT_RANDOM_INTERVAL_SEC
  jitterPercent.value = DEFAULT_JITTER_PERCENT
  visionEnabled.value = false
  visionIntervalSec.value = DEFAULT_VISION_INTERVAL_SEC
  try {
    localStorage.setItem(PET_MASTER_KEY, 'true')
    localStorage.setItem(PET_RANDOM_ENABLED_KEY, 'true')
    localStorage.setItem(PET_RANDOM_INTERVAL_KEY, String(DEFAULT_RANDOM_INTERVAL_SEC))
    localStorage.setItem(PET_RANDOM_JITTER_KEY, String(DEFAULT_JITTER_PERCENT))
    localStorage.setItem(PET_VISION_ENABLED_KEY, 'false')
    localStorage.setItem(PET_VISION_INTERVAL_KEY, String(DEFAULT_VISION_INTERVAL_SEC * 1000))
  } catch (_) {}
  Promise.resolve().then(() => { suppressWatch = false })
  emitConfigChanged()
}

function syncSettingFromStorage(event) {
  if (event.key === PET_VISION_HISTORY_KEY) {
    if (showHistory.value) refreshHistory()
    return
  }

  if (event.key === STORAGE_KEY) {
    aiConfigVersion.value += 1
    void refreshAiConfigStatus().then((applied) => {
      if (applied) correctVisionEnabled()
    })
    return
  }

  suppressWatch = true
  if (event.key === PET_MASTER_KEY) masterEnabled.value = loadMasterEnabled()
  else if (event.key === PET_RANDOM_ENABLED_KEY) randomEnabled.value = loadRandomEnabled()
  else if (event.key === PET_RANDOM_INTERVAL_KEY) randomIntervalSec.value = loadRandomIntervalSec()
  else if (event.key === PET_RANDOM_JITTER_KEY) jitterPercent.value = loadJitterPercent()
  else if (event.key === PET_VISION_ENABLED_KEY) visionEnabled.value = loadVisionEnabled()
  else if (event.key === PET_VISION_INTERVAL_KEY) visionIntervalSec.value = loadVisionIntervalSec()
  else {
    suppressWatch = false
    return
  }
  Promise.resolve().then(() => { suppressWatch = false })
}

onMounted(() => {
  window.addEventListener('storage', syncSettingFromStorage)
  void loadToolbarShortcutStatus()
  void refreshAiConfigStatus().then((applied) => {
    if (applied) correctVisionEnabled()
  })
})
onUnmounted(() => window.removeEventListener('storage', syncSettingFromStorage))
</script>

<style scoped>
.m-pet { display: grid; grid-template-columns: minmax(0, 1fr); gap: 12px; align-content: start; width: min(720px, calc(100vw - 32px)); max-height: min(640px, calc(100vh - 32px)); box-sizing: border-box; padding: 16px; overflow-y: auto; color: var(--md-sys-color-on-surface); }
.m-pet__head { display: flex; align-items: center; gap: 8px; } .m-pet__head h2 { margin: 0; margin-inline-end: auto; font: var(--m-title-large); }
.m-pet__content { display: grid; grid-template-columns: minmax(0, 1fr); gap: 16px; }
.m-pet__set { display: grid; grid-template-columns: minmax(0, 1fr); gap: 12px; }
.m-pet__row { display: flex; justify-content: flex-end; }
.m-pet__act { font: var(--m-label-large); color: var(--md-sys-color-primary-text, var(--md-sys-color-primary)); }
</style>
