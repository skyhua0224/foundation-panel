// Per-key desktop locale merge. zh_TW reads zh_TW, then zh, then en. Every other code reads itself, then en.
// zh_TW is a code of its own. An empty string does not replace a real one.

export const DESKTOP_LOCALES = ['en', 'zh', 'zh_TW', 'en_GB', 'en_US', 'de', 'fr', 'es', 'it', 'ja', 'ko', 'ru', 'pt_BR', 'pt', 'bg', 'cs', 'pl', 'sv', 'tr', 'uk']

export const LOCALE_NAMES = {
  en: 'English',
  en_GB: 'English (UK)',
  en_US: 'English (US)',
  zh: '简体中文',
  zh_TW: '繁體中文',
  de: 'Deutsch',
  fr: 'Français',
  es: 'Español',
  it: 'Italiano',
  ja: '日本語',
  ko: '한국어',
  ru: 'Русский',
  uk: 'Українська',
  pt: 'Português',
  pt_BR: 'Português (Brasil)',
  pl: 'Polski',
  sv: 'Svenska',
  tr: 'Türkçe',
  cs: 'Čeština',
  bg: 'Български',
}

const isObject = (v) => !!v && typeof v === 'object' && !Array.isArray(v)
const present = (v) => v !== undefined && v !== null && v !== ''

export function coversTexts(full, node) {
  if (Array.isArray(full)) {
    if (!Array.isArray(node) || node.length < full.length) return false
    return full.every((item, i) => coversTexts(item, node[i]))
  }
  if (isObject(full)) {
    if (!isObject(node)) return false
    return Object.keys(full).every((key) => coversTexts(full[key], node[key]))
  }
  if (!present(full)) return true
  return present(node)
}

export function mergeTexts(base, overlay) {
  if (Array.isArray(base) && Array.isArray(overlay)) {
    if (overlay.length === 0) return base
    let changed = false
    const out = base.map((item, i) => {
      if (i >= overlay.length) return item
      const next = mergeTexts(item, overlay[i])
      if (next !== item) changed = true
      return next
    })
    if (overlay.length > base.length) {
      for (let i = base.length; i < overlay.length; i++) {
        if (present(overlay[i]) || isObject(overlay[i]) || Array.isArray(overlay[i])) {
          out.push(overlay[i])
          changed = true
        }
      }
    }
    return changed ? out : base
  }
  if (isObject(base) && isObject(overlay)) {
    let changed = false
    const out = { ...base }
    for (const key of Object.keys(overlay)) {
      const value = overlay[key]
      if (!present(value) && !isObject(value) && !Array.isArray(value)) continue
      if (!(key in base)) {
        out[key] = value
        changed = true
        continue
      }
      const next = mergeTexts(base[key], value)
      if (next !== base[key]) {
        out[key] = next
        changed = true
      }
    }
    return changed ? out : base
  }
  if (!present(overlay)) return base
  return overlay === base ? base : overlay
}

/** Map a stored or system tag onto one of the 20 desktop codes. zh_TW stays zh_TW; anything else unknown becomes en. */
export function normalizeDesktopLocale(locale) {
  const tag = String(locale ?? '').trim().replace(/-/g, '_')
  if (!tag) return 'en'
  if (DESKTOP_LOCALES.includes(tag)) return tag
  const lower = tag.toLowerCase()
  if (lower.startsWith('zh') && /(_tw|_hk|_mo|hant)/.test(lower)) return 'zh_TW'
  if (lower.startsWith('zh')) return 'zh'
  const exact = DESKTOP_LOCALES.find((code) => code.toLowerCase() === lower)
  if (exact) return exact
  const [lang, region] = tag.split('_')
  if (region && region.length === 2) {
    const withRegion = lang.toLowerCase() + '_' + region.toUpperCase()
    if (DESKTOP_LOCALES.includes(withRegion)) return withRegion
  }
  const primary = (lang || '').toLowerCase()
  return DESKTOP_LOCALES.includes(primary) ? primary : 'en'
}

/** The message table for a desktop code. */
export function mergeLocale(en, zh, locale, partial) {
  const code = normalizeDesktopLocale(locale)
  const chinese = coversTexts(en, zh) ? zh : mergeTexts(en, zh)
  if (code === 'zh') return chinese
  if (code === 'zh_TW') return mergeTexts(chinese, partial)
  if (code === 'en') return en
  return mergeTexts(en, partial)
}

export function documentLang(locale) {
  const code = normalizeDesktopLocale(locale)
  if (code === 'zh') return 'zh-CN'
  return code.replace(/_/g, '-')
}
