// Nothing of the page may show that a browser is under it: no URL ghost on a drag, no browser key, no browser menu, no zoom, no selected chrome.
// One classic script for every page of the panel: the panel windows bundle it (import '../../src-tauri/inject-traces.js'), the framed Web pages get it from the
// proxy in front of inject-script.js. The engine side (status bubble, error page, accelerator keys, new windows, downloads) is windows.rs `harden_browser_surface`.
;(function (root) {
  'use strict'
  const doc = root.document
  if (!doc || root.__noBrowserTraces) return

  const EDITABLE = 'input:not([type=button]):not([type=submit]):not([type=reset]):not([type=checkbox]):not([type=radio]):not([type=range]):not([type=color]):not([type=file]):not([type=image]), textarea, [contenteditable=""], [contenteditable="true"]'

  const elementOf = (node) => (node && node.nodeType === 3 ? node.parentElement : node) || null
  const closest = (node, selector) => {
    const element = elementOf(node)
    return element && element.closest ? element.closest(selector) : null
  }
  const isEditable = (node) => !!closest(node, EDITABLE)

  // A drag starts only from an element that asked for it (draggable="true": tile and cover rearranging) or from a text field (its own selected text).
  // A link or an image drags by default and the drag ghost shows its URL, so those are cancelled; the CSS below already takes them out of the default.
  const dragIsOurs = (target) => {
    const element = elementOf(target)
    if (!element) return false
    // a link or an image drags by default; it stays a source only when it says draggable="true" itself (a tile around it does not make it one)
    const own = element.closest ? element.closest('a, img') : null
    if (own && own.getAttribute('draggable') !== 'true') return false
    const explicit = element.closest ? element.closest('[draggable="true"]') : null
    if (explicit) return true
    return isEditable(element)
  }

  const onDragStart = (event) => {
    if (!dragIsOurs(event.target)) event.preventDefault()
  }

  const hasSelection = () => {
    try {
      const selection = root.getSelection && root.getSelection()
      return !!selection && !selection.isCollapsed && String(selection).length > 0
    } catch {
      return false
    }
  }

  // The native menu is cancelled except over a text field or a selection; there windows.rs leaves cut, copy, paste, select all, undo and redo.
  // Bubble phase, so a menu of the page itself (which calls preventDefault) comes first and is left alone.
  const onContextMenu = (event) => {
    if (event.defaultPrevented) return
    if (isEditable(event.target) || hasSelection()) return
    event.preventDefault()
  }

  const devKeys = () => root.__devBuild === true || root.TAURI_PRODUCTION === false

  // The keys of a browser. Ctrl+K (our search), Ctrl+C/V/X/A/Z/Y and the arrows are not here.
  const browserKeyHit = (event) => {
    const key = String(event.key || '').toLowerCase()
    const ctrl = event.ctrlKey || event.metaKey
    if (key === 'f5' || key === 'f3' || key === 'f7') return true
    if (key === 'f12') return !devKeys()
    if (ctrl && event.shiftKey && ['i', 'j', 'c'].includes(key)) return !devKeys()
    if (ctrl && !event.altKey && ['r', 'p', 'f', 'g', 'u', 's', 'o', 'n', 't', 'd', 'h', 'j', 'l', 'e', '0', '+', '=', '-', '_'].includes(key)) {
      // Ctrl+Shift+N/T/R and friends are browser keys too; Ctrl+Shift+K and the editing keys are not in the list.
      return true
    }
    // Back and forward by keyboard: only when the page did not take the key for itself (the check is made after the page's handlers, in the bubble phase).
    if (event.altKey && !ctrl && (key === 'arrowleft' || key === 'arrowright') && !event.defaultPrevented) return true
    if (key === 'browserback' || key === 'browserforward' || key === 'browserrefresh' || key === 'browsersearch' || key === 'browserfavorites' || key === 'browserhome' || key === 'browserstop') return true
    return false
  }
  const onKeyDown = (event) => {
    if (browserKeyHit(event)) {
      event.preventDefault()
    }
  }

  // A file dropped on a place that takes no drop would be opened by the engine (the window navigates to it: a PDF or an image shows up in a browser view).
  // The places that take a drop (the theme editor, the file transfer) cancel dragover and drop themselves; what is left over gets the no-drop cursor and no navigation.
  const onDragOver = (event) => {
    if (event.defaultPrevented) return
    event.preventDefault()
    if (event.dataTransfer) event.dataTransfer.dropEffect = 'none'
  }
  const onDrop = (event) => {
    if (!event.defaultPrevented) event.preventDefault()
  }

  // Ctrl+wheel and a pinch (Chromium sends a pinch as Ctrl+wheel) zoom the page.
  const onWheel = (event) => {
    if (event.ctrlKey || event.metaKey) event.preventDefault()
  }
  const stop = (event) => event.preventDefault()

  // A field must not show the squiggles of the spell checker, the bubble of the saved values or a correction.
  const quiet = (event) => {
    const field = closest(event.target, EDITABLE)
    if (!field) return
    if (field.getAttribute('spellcheck') !== 'false') field.setAttribute('spellcheck', 'false')
    if (!field.hasAttribute('autocomplete')) field.setAttribute('autocomplete', 'off')
    if (!field.hasAttribute('autocorrect')) field.setAttribute('autocorrect', 'off')
    if (!field.hasAttribute('autocapitalize')) field.setAttribute('autocapitalize', 'off')
  }

  // Low specificity (:where) so a component that wants otherwise says so. The chrome of the page does not select (rail, tabs, buttons, titles, labels, tiles,
  // the caption bar); what people copy (addresses, codes, logs, values, text fields, table cells) stays selectable and is listed last so it wins inside a chrome.
  const CHROME = [
    'button', 'summary', 'label', 'legend', 'nav', 'aside', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'th',
    '[role=button]', '[role=tab]', '[role=tablist]', '[role=menu]', '[role=menuitem]', '[role=option]', '[role=switch]', '[role=tooltip]', '[role=navigation]', '[role=toolbar]',
    '.btn', '.nav', '.nav-link', '.navbar', '.dropdown-item', '.dropdown-menu', '.badge', '.card-header', '.card-title', '.modal-title', '.modal-header', '.form-label', '.form-check-label',
    '.list-group-item-action', '.tooltip', '.toast-header',
    '[class*="m-rail"]', '[class*="m-navlist"]', '[class*="m-nav__"]', '[class*="m-caption"]', '[class*="m-btn"]', '[class*="m-tile"]', '[class*="m-dash__"]', '[class*="m-chip"]',
    '[class*="m-tab"]', '[class*="m-seg"]', '[class*="m-menu"]',
    '[class*="panel-caption"]', '[class*="panel-shell"]', '[data-tauri-drag-region]',
  ]
  const VALUES = [
    'input', 'textarea', 'select', '[contenteditable]', 'pre', 'code', 'samp', 'kbd', 'output', 'td', 'dd',
    '[data-selectable]', '.selectable', '.user-select-all', '.font-monospace', '.form-control',
    '[class*="m-code"]', '[class*="m-log"]', '[class*="m-value"]', '[class*="m-copy"]', '[class*="m-mono"]', '[class*="m-addr"]', '[class*="m-field"]', '[class*="m-input"]',
  ]
  const CSS = [
    // a link, an image or a graphic is not a drag source unless it asks (draggable="true")
    ':where(a, img, svg, canvas, video, picture):not([draggable="true"]) { -webkit-user-drag: none; user-drag: none; }',
    ':where(html) { cursor: default; -webkit-tap-highlight-color: transparent; -webkit-touch-callout: none; }',
    ':where(*) { -webkit-tap-highlight-color: transparent; }',
    `:where(${CHROME.join(', ')}) { -webkit-user-select: none; user-select: none; }`,
    `:where(${VALUES.join(', ')}) { -webkit-user-select: text; user-select: text; }`,
    ':where(input, textarea)::-webkit-search-cancel-button { cursor: default; }',
  ].join('\n')

  const addStyle = () => {
    if (doc.querySelector('style[data-no-browser-traces]')) return
    const style = doc.createElement('style')
    style.setAttribute('data-no-browser-traces', '')
    style.textContent = CSS
    ;(doc.head || doc.documentElement).appendChild(style)
  }

  const install = () => {
    root.__noBrowserTraces = { dragIsOurs, browserKeyHit, CSS }
    doc.addEventListener('dragstart', onDragStart, true)
    doc.addEventListener('dragover', onDragOver, false)
    doc.addEventListener('drop', onDrop, false)
    doc.addEventListener('contextmenu', onContextMenu, false)
    root.addEventListener('keydown', onKeyDown, false)
    root.addEventListener('wheel', onWheel, { capture: true, passive: false })
    for (const type of ['gesturestart', 'gesturechange', 'gestureend']) doc.addEventListener(type, stop, { capture: true, passive: false })
    doc.addEventListener('focusin', quiet, true)
    // an auxiliary click (middle button) on a link opens a window in a browser
    doc.addEventListener('auxclick', (event) => { if (event.button === 1 && closest(event.target, 'a[href]')) event.preventDefault() }, true)
    addStyle()
  }
  install()
})(typeof window !== 'undefined' ? window : globalThis)
