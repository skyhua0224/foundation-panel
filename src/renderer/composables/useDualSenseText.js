import { computed } from 'vue'
import { useI18n } from '../desktop/i18n/index.js'
import { formatText } from './dualsenseText.js'

const lookup = (root, path) => String(path).split('.').reduce((node, part) => node?.[part], root)

export function useDualSenseText() {
  const { t, locale } = useI18n()
  const text = computed(() => t.value.dualSense)
  /** Sentence of the status card and error maps: `status.states.ready`. */
  const tx = (path, params) => formatText(lookup(text.value, path) ?? path, params)
  /** Sentence of the tuning page: `presets.title` under `dualSense.tuning`. */
  const tr = (path, params) => formatText(lookup(text.value.tuning, path) ?? path, params)
  return { text, tx, tr, locale }
}
