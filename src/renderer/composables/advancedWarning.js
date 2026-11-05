// Warning shown before an advanced controller section opens. The "do not show
// again" choice lives in the panel's own storage and covers every advanced
// section of the controller pages.
export const ADVANCED_WARNING_HIDDEN_KEY = 'ds5.advanced.warning.hidden'
export const ADVANCED_WARNING_SECONDS = 5

/** True when the user chose not to see the warning again. */
export function isAdvancedWarningHidden(storage) {
  try {
    return storage?.getItem(ADVANCED_WARNING_HIDDEN_KEY) === '1'
  } catch {
    // Storage can be unavailable; the warning then always shows.
    return false
  }
}

export function setAdvancedWarningHidden(storage, hidden) {
  try {
    if (hidden) storage?.setItem(ADVANCED_WARNING_HIDDEN_KEY, '1')
    else storage?.removeItem(ADVANCED_WARNING_HIDDEN_KEY)
  } catch {
    // Without storage the choice only lasts until the panel reloads.
  }
}

/** The panel's storage, or null when the webview refuses access to it. */
export function panelStorage() {
  try {
    return window.localStorage
  } catch {
    return null
  }
}

/**
 * Whole seconds left on the confirm countdown, rounded up so the button shows
 * the full count first and unlocks exactly when the time is over.
 */
export function remainingSeconds(elapsedMs, total = ADVANCED_WARNING_SECONDS) {
  return Math.max(0, Math.ceil(total - Math.max(0, elapsedMs) / 1000))
}
