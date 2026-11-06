import zh from './shortcuts.md?raw'
import en from './shortcuts.en.md?raw'

/** The shortcut manual in the language of the window (zh: Chinese; anything else: English, as the rest of the desktop falls back). */
export const shortcutsSource = (locale) => (locale === 'zh' ? zh : en)
