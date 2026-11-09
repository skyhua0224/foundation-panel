import { ref, computed, watch } from 'vue'
import { LOCALE_SYSTEM, choiceOf, readLocalePref, resolveSystemLocale, watchSystemLanguage, writeLocalePref } from 'foundation-ui/locale/localePref.js'
import { zh } from './zh.js'
import { en } from './en.js'
import { DESKTOP_LOCALES, LOCALE_NAMES, documentLang, mergeLocale, normalizeDesktopLocale } from './merge.js'
import { zh_TW } from './zh_TW.js'
import { en_GB } from './en_GB.js'
import { en_US } from './en_US.js'
import { de } from './de.js'
import { fr } from './fr.js'
import { es } from './es.js'
import { it } from './it.js'
import { ja } from './ja.js'
import { ko } from './ko.js'
import { ru } from './ru.js'
import { pt_BR } from './pt_BR.js'
import { pt } from './pt.js'
import { bg } from './bg.js'
import { cs } from './cs.js'
import { pl } from './pl.js'
import { sv } from './sv.js'
import { tr } from './tr.js'
import { uk } from './uk.js'

export { DESKTOP_LOCALES, LOCALE_NAMES, normalizeDesktopLocale }
const bundles = { zh_TW, en_GB, en_US, de, fr, es, it, ja, ko, ru, pt_BR, pt, bg, cs, pl, sv, tr, uk }
const merged = new Map()
const messagesFor = (locale) => {
  const code = normalizeDesktopLocale(locale)
  if (merged.has(code)) return merged.get(code)
  const table = mergeLocale(en, zh, code, bundles[code])
  merged.set(code, table)
  return table
}
const LOCALE_REQUEST_KEY = 'foundation-desktop-locale-request'
let localLocaleRequestId = null
let latestBackendLocaleRevision = 0

const systemDesktopLocale = () => normalizeDesktopLocale(resolveSystemLocale(typeof navigator !== 'undefined' ? (navigator.languages?.length ? navigator.languages : [navigator.language]) : [], DESKTOP_LOCALES))
function storedExplicit() {
  try { const saved = localStorage.getItem('language'); return DESKTOP_LOCALES.includes(saved) ? saved : '' } catch { return '' }
}

function getDefaultLocale() {
  // the stored choice first: 跟随系统 gives the language of the system, a code is that language
  const pref = readLocalePref()
  if (pref === LOCALE_SYSTEM) return systemDesktopLocale()
  if (pref) return normalizeDesktopLocale(pref)
  try {
    const saved = localStorage.getItem('language')
    if (DESKTOP_LOCALES.includes(saved)) return saved
  } catch {
    // Fall back to the browser locale when storage is unavailable.
  }

  const browserLang = navigator.language || navigator.userLanguage || ''
  return normalizeDesktopLocale(browserLang)
}

const currentLocale = ref(getDefaultLocale())
// What the person chose: 跟随系统 (the default) or a language; `system` is stored as such and followed live
const currentChoice = ref(choiceOf(readLocalePref(), storedExplicit(), systemDesktopLocale()))
let localeRevision = 0

function updateCurrentLocale(locale, persist = false) {
  const normalized = normalizeDesktopLocale(locale)
  localeRevision += 1
  if (normalized !== currentLocale.value) {
    currentLocale.value = normalized
  }
  if (persist) {
    try {
      localStorage.setItem('language', normalized)
    } catch {
      // Keep the in-memory locale when storage is unavailable.
    }
  }
}

function createLocaleRequestId() {
  try {
    return crypto.randomUUID()
  } catch {
    return `${Date.now()}-${Math.random().toString(36).slice(2)}`
  }
}

function setLatestLocaleRequest(requestId) {
  localLocaleRequestId = requestId
  try {
    localStorage.setItem(LOCALE_REQUEST_KEY, requestId)
  } catch {
    // The originating webview can still reject its own stale confirmations.
  }
}

function getLatestLocaleRequest() {
  try {
    return localStorage.getItem(LOCALE_REQUEST_KEY)
  } catch {
    return localLocaleRequestId
  }
}

function clearLatestLocaleRequest() {
  localLocaleRequestId = null
  try {
    localStorage.removeItem(LOCALE_REQUEST_KEY)
  } catch {
    // Ignore when storage is unavailable.
  }
}

function setDocumentLanguage(locale) {
  document.documentElement.lang = documentLang(locale)
}

watch(currentLocale, setDocumentLanguage, { immediate: true })

// 启动时恢复桌面 UI；托盘会在创建菜单前从同一个 locale 同步初始化。
let syncInitialized = false
async function syncLocaleFromSunshine() {
  if (syncInitialized) return
  syncInitialized = true
  const revision = localeRevision
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    const config = await invoke('parse_sunshine_config')
    if (!config?.locale || revision !== localeRevision || currentChoice.value === LOCALE_SYSTEM) return
    // The host stores a code such as zh_TW; the desktop accepts all 20 and keeps zh_TW as zh_TW.
    const guiLocale = normalizeDesktopLocale(config.locale)
    updateCurrentLocale(guiLocale, true)
  } catch {
    // 非 Tauri 环境或 API 不可用，忽略
  }
}
syncLocaleFromSunshine()

// 监听托盘语言切换事件
async function listenTrayLocaleChanged() {
  try {
    const { listen } = await import('@tauri-apps/api/event')
    listen('tray-locale-changed', (event) => {
      const payload = event.payload
      const eventLocale = typeof payload === 'string' ? payload : payload?.locale
      if (!eventLocale) return

      const source = typeof payload === 'object' ? payload.source : 'tray'
      const requestId = typeof payload === 'object' ? payload.requestId : null
      const revision = typeof payload === 'object' ? Number(payload.revision) : Number.NaN
      if (Number.isSafeInteger(revision)) {
        if (revision <= latestBackendLocaleRevision) return
        latestBackendLocaleRevision = revision
      }
      if (source === 'frontend' && requestId) {
        const latestRequestId = getLatestLocaleRequest()
        if (latestRequestId && latestRequestId !== requestId) return
      } else if (source === 'tray') {
        clearLatestLocaleRequest()
        // a language picked in the tray menu is an explicit choice of the person
        currentChoice.value = normalizeDesktopLocale(eventLocale); writeLocalePref(currentChoice.value)
      }

      updateCurrentLocale(eventLocale, true)
      window.dispatchEvent(new CustomEvent('locale-changed', {
        detail: { locale: normalizeDesktopLocale(eventLocale) },
      }))
    })
  } catch {
    // 非 Tauri 环境，忽略
  }
}
listenTrayLocaleChanged()

// localStorage is shared by the app webviews, but each webview owns its Vue
// state. Keep already-open toolbar and tool windows in sync with another
// window changing the language.
window.addEventListener('storage', (event) => {
  if (event.key !== 'language' || !event.newValue) return
  updateCurrentLocale(event.newValue)
})

// 用户明确切换时，通过同一条命令更新 Sunshine UI 和托盘语言。
async function syncLocalePreferences(locale, requestId) {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    await invoke('set_locale_preferences', { locale, requestId })
  } catch (error) {
    console.warn('Failed to sync locale preferences:', error)
  }
}

function requestLocaleChange(value) {
  const normalized = normalizeDesktopLocale(value)
  if (normalized === currentLocale.value) return

  const requestId = createLocaleRequestId()
  setLatestLocaleRequest(requestId)
  updateCurrentLocale(normalized, true)
  syncLocalePreferences(normalized, requestId)
}

/** The choice of the language: 'system' or one of the 20 codes. 'system' is stored as such; the language of the system is applied (the host and the tray follow it) and followed when it changes. */
export function setLocaleChoice(choice) {
  if (choice === LOCALE_SYSTEM) {
    currentChoice.value = LOCALE_SYSTEM
    writeLocalePref(LOCALE_SYSTEM)
    requestLocaleChange(systemDesktopLocale())
    return
  }
  const code = normalizeDesktopLocale(choice)
  currentChoice.value = code
  writeLocalePref(code)
  requestLocaleChange(code)
}

// the operating system changes its language: a window that follows it changes with it
watchSystemLanguage(() => { if (currentChoice.value === LOCALE_SYSTEM) requestLocaleChange(systemDesktopLocale()) })

export function useI18n() {
  const t = computed(() => messagesFor(currentLocale.value))
  const choice = computed(() => currentChoice.value)
  const systemLocale = computed(() => systemDesktopLocale())
  const locale = computed({
    get: () => currentLocale.value,
    set: (val) => {
      setLocaleChoice(val)
    },
  })
  const toggleLocale = () => {
    const newLocale = locale.value === 'zh' ? 'en' : 'zh'
    locale.value = newLocale
  }
  return { t, locale, toggleLocale, choice, setChoice: setLocaleChoice, systemLocale }
}

if (currentChoice.value === LOCALE_SYSTEM) {
  try {
    if (localStorage.getItem('language') !== currentLocale.value) { localStorage.setItem('language', currentLocale.value); syncLocalePreferences(currentLocale.value, createLocaleRequestId()) }
  } catch { /* storage unavailable */ }
}
