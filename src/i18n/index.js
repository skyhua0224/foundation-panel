import { ref, watch } from 'vue'
import { LOCALE_SYSTEM, choiceOf, readLocalePref, resolveSystemLocale, watchSystemLanguage, writeLocalePref } from 'foundation-ui/locale/localePref.js'
import en from './locales/en.js'
import en_GB from './locales/en_GB.js'
import en_US from './locales/en_US.js'
import zh from './locales/zh.js'
import zh_TW from './locales/zh_TW.js'
import de from './locales/de.js'
import fr from './locales/fr.js'
import es from './locales/es.js'
import it from './locales/it.js'
import ja from './locales/ja.js'
import ko from './locales/ko.js'
import ru from './locales/ru.js'
import uk from './locales/uk.js'
import pt from './locales/pt.js'
import pt_BR from './locales/pt_BR.js'
import pl from './locales/pl.js'
import sv from './locales/sv.js'
import tr from './locales/tr.js'
import cs from './locales/cs.js'
import bg from './locales/bg.js'

// 方案乙（去 vue-i18n 依赖、保留 20 语言数据）：welcome 页只用到
// t(key) 平铺查表 + 语言切换，这里保持原对外行为（en 兜底、ref 响应式）。
export const supportedLocales = [
  { code: 'en', name: 'English' },
  { code: 'en_GB', name: 'English (UK)' },
  { code: 'en_US', name: 'English (US)' },
  { code: 'zh', name: '简体中文' },
  { code: 'zh_TW', name: '繁體中文' },
  { code: 'de', name: 'Deutsch' },
  { code: 'fr', name: 'Français' },
  { code: 'es', name: 'Español' },
  { code: 'it', name: 'Italiano' },
  { code: 'ja', name: '日本語' },
  { code: 'ko', name: '한국어' },
  { code: 'ru', name: 'Русский' },
  { code: 'uk', name: 'Українська' },
  { code: 'pt', name: 'Português' },
  { code: 'pt_BR', name: 'Português (Brasil)' },
  { code: 'pl', name: 'Polski' },
  { code: 'sv', name: 'Svenska' },
  { code: 'tr', name: 'Türkçe' },
  { code: 'cs', name: 'Čeština' },
  { code: 'bg', name: 'Български' },
]

const messages = {
  en, en_GB, en_US, zh, zh_TW, de, fr, es, it, ja,
  ko, ru, uk, pt, pt_BR, pl, sv, tr, cs, bg,
}

const systemLanguages = () => (typeof navigator !== 'undefined' ? (navigator.languages?.length ? navigator.languages : [navigator.language || navigator.userLanguage]) : [])
const resolveSystem = () => resolveSystemLocale(systemLanguages(), supportedLocales.map((l) => l.code))
const locale = ref(getDefaultLocale())
// What the person chose: 跟随系统 (the default) or a language. `system` is what is stored; the language it resolves to is followed live (the operating system's language changes while the window is open)
export const localeChoice = ref(choiceOf(readLocalePref(), explicitLocale(), resolveSystem()))

function setDocumentLanguage(value) {
  if (typeof document === 'undefined') return
  // 语言码即 BCP 47 标签（en_GB → en-GB、zh_TW → zh-TW）；zh 特判为
  // zh-CN 保留屏幕阅读器的变体提示，未知值兜底 en。
  const tag = String(value || 'en').replace(/_/g, '-')
  document.documentElement.lang = tag === 'zh' ? 'zh-CN' : tag
}
watch(locale, setDocumentLanguage, { immediate: true })

// the language the panel had stored before the choice existed (an explicit one), '' when there is none
function explicitLocale() {
  for (const key of ['sunshine-locale', 'language']) {
    try { const saved = localStorage.getItem(key); if (saved && supportedLocales.find((l) => l.code === saved)) return saved } catch { /* storage unavailable */ }
  }
  return ''
}

// 获取默认语言
export function getDefaultLocale() {
  // the stored choice first: 跟随系统 gives the language of the system, a code is that language
  const pref = readLocalePref()
  if (pref === LOCALE_SYSTEM) return resolveSystem()
  if (pref && supportedLocales.find((l) => l.code === pref)) return pref

  // 尝试从 localStorage 读取
  const saved = localStorage.getItem('sunshine-locale')
  if (saved && supportedLocales.find((l) => l.code === saved)) {
    return saved
  }

  // 面板自己的语言设置（desktop/i18n 使用的 'language'）优先于浏览器语言
  const panelLang = localStorage.getItem('language')
  if (panelLang && supportedLocales.find((l) => l.code === panelLang)) {
    return panelLang
  }

  // 尝试从浏览器语言获取
  const browserLang = navigator.language || navigator.userLanguage
  const langCode = browserLang.split('-')[0]
  const matched = supportedLocales.find((l) => l.code === langCode || l.code === browserLang)
  if (matched) {
    return matched.code
  }

  // 默认返回英文，避免英语系统用户首次打开时看到中文界面
  return 'en'
}

// 设置语言: a language of the person's own (the choice becomes that code)
export function setLocale(value) {
  if (!supportedLocales.find((l) => l.code === value)) return false
  locale.value = value
  localeChoice.value = value
  localStorage.setItem('sunshine-locale', value)
  writeLocalePref(value)
  return true
}

/** The choice of the language, applied at once: 'system' (stored as such, the language of the system is shown and followed) or a code. */
export function setLocaleChoice(choice) {
  if (choice === LOCALE_SYSTEM) {
    localeChoice.value = LOCALE_SYSTEM
    writeLocalePref(LOCALE_SYSTEM)
    locale.value = resolveSystem()
    return true
  }
  return setLocale(choice)
}

/** The language the system resolves to for this window (the code), for the line 跟随系统 · 简体中文. */
export const systemLocale = () => resolveSystem()

// the operating system changes its language: a window that follows it changes with it
watchSystemLanguage(() => { if (localeChoice.value === LOCALE_SYSTEM) locale.value = resolveSystem() })

// 平铺查表：当前语言缺失的键回退到 en，再缺失则原样返回键名。
// Per key: zh_TW then zh then en; every other language then en. An empty string does not count.
// A key none of them have is returned as itself.
export function t(key) {
  const own = messages[locale.value]?.[key]
  if (own !== undefined && own !== null && own !== '') return own
  if (locale.value === 'zh_TW') {
    const traditional = messages.zh?.[key]
    if (traditional !== undefined && traditional !== null && traditional !== '') return traditional
  }
  const english = messages.en?.[key]
  if (english !== undefined && english !== null && english !== '') return english
  return key
}

export function useWelcomeI18n() {
  return { t, locale }
}
