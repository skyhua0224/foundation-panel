// The keyboard shortcuts of the unified search. The shared search opens on "/" when no field has focus, but it judges the field by the
// event target, and a Material text field keeps its <input> in a shadow root: outside it the target is the host element (md-outlined-text-field),
// which is not an INPUT, so every "/" typed into such a field (the "//" of https://) opened the search and took the focus. The guard here looks
// through the whole event path, and it also keeps the key while an input method is composing. Ctrl/Cmd+K and Ctrl/Cmd+F are not touched.

const FIELD_TAGS = /^(INPUT|TEXTAREA|SELECT)$/
const FIELD_HOSTS = /^MD-(OUTLINED|FILLED)-(TEXT-FIELD|SELECT)$/
const FIELD_ROLES = new Set(['textbox', 'combobox', 'searchbox', 'spinbutton'])

function isFieldElement(el) {
  if (!el || el.nodeType !== 1) return false
  const tag = String(el.tagName || '').toUpperCase()
  if (el.isContentEditable || FIELD_TAGS.test(tag) || FIELD_HOSTS.test(tag)) return true
  const role = typeof el.getAttribute === 'function' ? el.getAttribute('role') : null
  return !!role && FIELD_ROLES.has(role)
}

/** The element that has the keyboard focus, through open shadow roots. */
function deepActiveElement(doc) {
  let el = doc?.activeElement || null
  while (el?.shadowRoot?.activeElement) el = el.shadowRoot.activeElement
  return el
}

/** True when the key is typed into an editable element: the event path (through shadow roots) or the focused element is a field. */
export function isEditableTarget(event, doc = typeof document !== 'undefined' ? document : null) {
  const path = typeof event?.composedPath === 'function' ? event.composedPath() : []
  if (path.some(isFieldElement)) return true
  if (isFieldElement(event?.target)) return true
  let el = event?.target
  while (el && el.nodeType === 1) {
    if (isFieldElement(el)) return true
    el = el.parentElement
  }
  return isFieldElement(deepActiveElement(doc))
}

/** True while an input method is composing (keyCode 229 is what some engines report for such a key without isComposing). */
export function isComposingKey(event) {
  return !!event?.isComposing || event?.keyCode === 229 || event?.key === 'Process'
}

/** True for the plain "/" key (no Ctrl, Cmd or Alt), the one that must stay a character in a field. */
export function isPlainSlash(event) {
  return event?.key === '/' && !event.ctrlKey && !event.metaKey && !event.altKey
}

/** The "/" shortcut must not fire from a field or while composing. */
export function shouldIgnoreSlash(event, doc) {
  return isPlainSlash(event) && (isComposingKey(event) || isEditableTarget(event, doc))
}

/**
 * The keydown listener for `search` (the result of useUnifiedSearch created with `shortcuts: false`): "/" in a field or during composition is left
 * to the page, every other shortcut (Ctrl/Cmd+K, Ctrl/Cmd+F, Esc) goes to the search as before.
 */
export function createSearchShortcutHandler(search, doc) {
  return (event) => {
    if (shouldIgnoreSlash(event, doc)) return false
    return search.onShortcut(event)
  }
}

/** Listen on the document (capture, like the shared search did); returns the function that removes the listener. */
export function installSearchShortcut(search, doc = document) {
  const handler = createSearchShortcutHandler(search, doc)
  doc.addEventListener('keydown', handler, true)
  return () => doc.removeEventListener('keydown', handler, true)
}
