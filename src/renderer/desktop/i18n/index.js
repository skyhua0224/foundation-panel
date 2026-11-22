import { ref, computed, watch } from 'vue'
import { LOCALE_SYSTEM, choiceOf, readLocalePref, resolveSystemLocale, watchSystemLanguage, writeLocalePref } from 'foundation-ui/locale/localePref.js'
import { withUsbipOwnership } from './usbipOwnership.js'
import { zh } from './zh.js'
import { en } from './en.js'
import { DESKTOP_LOCALES, LOCALE_NAMES, documentLang, mergeLocale, normalizeDesktopLocale } from './merge.js'
export { DESKTOP_LOCALES, LOCALE_NAMES, normalizeDesktopLocale }
// English and Chinese are the fallbacks of every table (merge.js), so they stay in the entry; every other language is a file of its own,
// read when the language is chosen (the window waits for it before it draws: i18nReady) and kept after.
const loaders = {
  zh_TW: () => import('./zh_TW.js').then((m) => m.zh_TW),
  en_GB: () => import('./en_GB.js').then((m) => m.en_GB),
  en_US: () => import('./en_US.js').then((m) => m.en_US),
  de: () => import('./de.js').then((m) => m.de),
  fr: () => import('./fr.js').then((m) => m.fr),
  es: () => import('./es.js').then((m) => m.es),
  it: () => import('./it.js').then((m) => m.it),
  ja: () => import('./ja.js').then((m) => m.ja),
  ko: () => import('./ko.js').then((m) => m.ko),
  ru: () => import('./ru.js').then((m) => m.ru),
  pt_BR: () => import('./pt_BR.js').then((m) => m.pt_BR),
  pt: () => import('./pt.js').then((m) => m.pt),
  bg: () => import('./bg.js').then((m) => m.bg),
  cs: () => import('./cs.js').then((m) => m.cs),
  pl: () => import('./pl.js').then((m) => m.pl),
  sv: () => import('./sv.js').then((m) => m.sv),
  tr: () => import('./tr.js').then((m) => m.tr),
  uk: () => import('./uk.js').then((m) => m.uk),
}
const bundles = {}
const pending = new Map()
const merged = new Map()
// Moves when a language arrives, so a table built from the fallbacks while it was missing is built again.
const bundleVersion = ref(0)
const hasBundle = (code) => code === 'en' || code === 'zh' || Object.hasOwn(bundles, code)
/** Reads the file of one language (nothing for en and zh); a failed read leaves the fallbacks in place and can be asked again. */
export function loadLocale(locale) {
  const code = normalizeDesktopLocale(locale)
  if (hasBundle(code) || !loaders[code]) return Promise.resolve(code)
  if (!pending.has(code)) {
    pending.set(code, loaders[code]().then((bundle) => {
      bundles[code] = bundle
      merged.delete(code)
      bundleVersion.value += 1
    }).catch((error) => {
      console.warn(`Language ${code} could not be loaded:`, error)
    }).finally(() => pending.delete(code)))
  }
  return pending.get(code).then(() => code)
}
const messagesFor = (locale) => {
  const code = normalizeDesktopLocale(locale)
  // read here so a table that is shown re-evaluates when its language arrives
  void bundleVersion.value
  if (merged.has(code)) return merged.get(code)
  const table = withUsbipOwnership(mergeLocale(en, zh, code, bundles[code]), code)
  // a table made without its language is the fallbacks only: it is not kept
  if (hasBundle(code)) merged.set(code, table)
  else void loadLocale(code)
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
/** Resolves when the language of this window is read: a window draws after it, so its first frame is already in its language. */
export const i18nReady = loadLocale(currentLocale.value)

// The language is applied once its file is read, so a switch never shows the other language in between; the stored choice is written at once.
function updateCurrentLocale(locale, persist = false) {
  const normalized = normalizeDesktopLocale(locale)
  localeRevision += 1
  const revision = localeRevision
  if (persist) {
    try {
      localStorage.setItem('language', normalized)
    } catch {
      // Keep the in-memory locale when storage is unavailable.
    }
  }
  // the language that is shown but whose file never arrived is read again
  if (normalized === currentLocale.value && hasBundle(normalized)) return Promise.resolve()
  if (hasBundle(normalized)) {
    currentLocale.value = normalized
    return Promise.resolve()
  }
  return loadLocale(normalized).then(() => {
    // a newer request wins: its own language is applied by it
    if (revision === localeRevision) currentLocale.value = normalized
  })
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

      updateCurrentLocale(eventLocale, true).then(() => {
        window.dispatchEvent(new CustomEvent('locale-changed', {
          detail: { locale: normalizeDesktopLocale(eventLocale) },
        }))
      })
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
  // choosing the language that is shown still cancels a switch whose file is on its way
  if (normalized === currentLocale.value && pending.size === 0 && hasBundle(normalized)) return

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
