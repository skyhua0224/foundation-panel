// What the person was doing in a page of the panel: where it is scrolled, which tab and which section are open, what was typed into its fields.
//
// The panel lets go of its main window when it has been hidden for a while (src-tauri/src/mem_adaptive.rs) and makes it again before it is needed. This script is the other half of that: it keeps a
// snapshot of the page up to date while the page is used and puts it back into the window that is made again, so that a window that was let go looks like one that was never let go.
//
// One script, two roles:
//   * in the panel's own window (the top document) it records the shell and the route, collects the state its frame reports, and hands the whole to the native side (`panel_state_save`, kept in memory
//     only: a draft never touches the disk); a window that is made again carries it in `window.__PANEL_RESTORE__` (src-tauri/src/view_state.rs) and the script applies it.
//   * in a page of the Web UI inside the frame (the proxy puts the script into every page) it records the same things and reports them to the panel with postMessage; the panel answers with the state
//     of that page when the page says it is ready (`panel-view-ready`).
//
// Nothing sensitive is kept: no password, hidden or file field, no field whose name, label or autocomplete hint says secret/token/PIN/OTP/card; a value is at most 4000 characters; a snapshot at most
// 200 fields. A restore never overwrites what the person did after the window came back (any pointer, key or wheel input ends it) and never touches a field that already holds the saved value.
;(function () {
  if (window.__panelViewState) return
  const MAX_FIELDS = 200
  const MAX_VALUE = 4000
  const MAX_TOGGLES = 100
  const SAVE_DELAY = 300
  const RESTORE_BUDGET = 4000
  const RESTORE_STEP = 120
  const SENSITIVE = /(^|[^a-z])(pass(word|wd|phrase)?|pwd|secret|token|api[-_ ]?key|private[-_ ]?key|pin|otp|cvv|cvc|credential|credit|card)([^a-z]|$)/i
  const SENSITIVE_AUTOCOMPLETE = /^(current-password|new-password|one-time-code|cc-)/i
  const TRUSTED_PARENTS = new Set([
    'http://tauri.localhost',
    'https://tauri.localhost',
    'tauri://localhost',
    'http://localhost:8080',
    'https://localhost:8080',
    'http://127.0.0.1:8080',
    'https://127.0.0.1:8080',
  ])

  // ---- pure helpers (tested in src/renderer/viewState.test.js) ----------------------------------------------------------------------------------------------------------------------------

  const volatileClass = (name) => /\d{3,}|^(is-|has-|v-|js-|ng-)|^(active|open|opened|show|shown|hover|focus|focused|selected|collapsed|expanded|disabled|hidden|visible|loading|dirty|touched)$/i.test(name)

  /** The parts that make an element findable again: its tag, id, name and the first two stable classes. */
  function keyParts(info) {
    const parts = { tag: String(info.tag || '*').toLowerCase() }
    if (info.id && !/\d{4,}/.test(info.id)) parts.id = info.id
    if (info.name) parts.name = info.name
    if (info.key) parts.key = info.key
    const classes = (info.classes || []).filter((name) => name && !volatileClass(name)).slice(0, 2)
    if (classes.length) parts.classes = classes
    return parts
  }

  const escapeIdent = (value) => String(value).replace(/[^a-zA-Z0-9_-]/g, (ch) => '\\' + ch)
  const escapeAttr = (value) => String(value).replace(/["\\]/g, '\\$&')

  /** A selector for `keyParts`: an id or an explicit key is enough alone; otherwise tag, name and classes, and the caller adds the index among the elements that match. */
  function selectorOf(parts) {
    if (parts.key) return '[data-scroll-key="' + escapeAttr(parts.key) + '"]'
    if (parts.id) return '#' + escapeIdent(parts.id)
    let selector = parts.tag
    if (parts.name) selector += '[name="' + escapeAttr(parts.name) + '"]'
    for (const name of parts.classes || []) selector += '.' + escapeIdent(name)
    return selector
  }

  function isSensitiveField(info) {
    const type = String(info.type || '').toLowerCase()
    if (type === 'password' || type === 'hidden' || type === 'file') return true
    if (SENSITIVE_AUTOCOMPLETE.test(String(info.autocomplete || ''))) return true
    const text = [info.name, info.id, info.label, info.placeholder, info.ariaLabel].filter(Boolean).join(' ')
    return SENSITIVE.test(text)
  }

  /** Whether a field is recorded: only what the person changed, never a secret, never an absurdly long value. */
  function recordableField(info) {
    if (!info.touched || isSensitiveField(info)) return false
    const type = String(info.type || '').toLowerCase()
    if (['button', 'submit', 'reset', 'image'].includes(type)) return false
    return String(info.value == null ? '' : info.value).length <= MAX_VALUE
  }

  /** The state of a frame page is for that page only: same path, same query. */
  function samePage(saved, now) {
    if (!saved || !now) return false
    return String(saved.path || '') === String(now.path || '')
  }

  /** A snapshot read from the native side or from a frame is data, never code: only the known shape is taken. */
  function sanitize(raw) {
    const out = { scroll: [], fields: [], tabs: [], toggles: [] }
    if (!raw || typeof raw !== 'object') return out
    const num = (value) => (Number.isFinite(value) ? Math.max(0, Math.round(value)) : 0)
    const parts = (value) => (value && typeof value === 'object' ? value : {})
    for (const item of Array.isArray(raw.scroll) ? raw.scroll.slice(0, 40) : []) out.scroll.push({ at: parts(item.at), top: num(item.top), left: num(item.left) })
    for (const item of Array.isArray(raw.fields) ? raw.fields.slice(0, MAX_FIELDS) : []) {
      if (typeof item.value === 'string' || typeof item.checked === 'boolean') out.fields.push({ at: parts(item.at), type: String(item.type || ''), value: typeof item.value === 'string' ? item.value.slice(0, MAX_VALUE) : undefined, checked: typeof item.checked === 'boolean' ? item.checked : undefined })
    }
    for (const item of Array.isArray(raw.tabs) ? raw.tabs.slice(0, MAX_TOGGLES) : []) out.tabs.push({ at: parts(item.at), label: String(item.label || '').slice(0, 80) })
    for (const item of Array.isArray(raw.toggles) ? raw.toggles.slice(0, MAX_TOGGLES) : []) out.toggles.push({ at: parts(item.at), label: String(item.label || '').slice(0, 80), expanded: item.expanded === true })
    out.path = typeof raw.path === 'string' ? raw.path.slice(0, 600) : ''
    return out
  }

  // ---- reading the document ---------------------------------------------------------------------------------------------------------------------------------------------------------------

  const infoOf = (el) => ({ tag: el.tagName, id: el.id || '', name: el.getAttribute && el.getAttribute('name') || '', key: el.getAttribute && (el.getAttribute('data-scroll-key') || '') || '', classes: Array.from(el.classList || []) })

  /** Where an element is among the elements of the document that match its selector. */
  function addressOf(el) {
    const parts = keyParts(infoOf(el))
    let index = 0
    try {
      const all = document.querySelectorAll(selectorOf(parts))
      index = Array.prototype.indexOf.call(all, el)
    } catch (e) { index = 0 }
    return { p: parts, i: Math.max(0, index) }
  }

  function find(address) {
    if (!address || !address.p) return null
    try {
      const all = document.querySelectorAll(selectorOf(address.p))
      return all[address.i] || null
    } catch (e) { return null }
  }

  const labelOf = (el) => {
    const text = (el.getAttribute && (el.getAttribute('aria-label') || '')) || (el.textContent || '')
    return String(text).replace(/\s+/g, ' ').trim().slice(0, 80)
  }

  const touched = new WeakSet()
  const scrolled = new Map()

  const fieldInfo = (el) => ({
    tag: el.tagName, type: el.type, name: el.getAttribute('name') || '', id: el.id || '', autocomplete: el.getAttribute('autocomplete') || '', placeholder: el.getAttribute('placeholder') || '',
    ariaLabel: el.getAttribute('aria-label') || '', label: el.labels && el.labels[0] ? el.labels[0].textContent : '', value: el.value, touched: touched.has(el),
  })

  function collect() {
    const state = { scroll: [], fields: [], tabs: [], toggles: [], path: location.pathname + location.search + location.hash }
    const root = document.scrollingElement
    if (root && (root.scrollTop || root.scrollLeft)) state.scroll.push({ at: { p: { tag: '#root' }, i: 0 }, top: Math.round(root.scrollTop), left: Math.round(root.scrollLeft) })
    for (const el of Array.from(scrolled.keys())) {
      if (!el.isConnected) { scrolled.delete(el); continue }
      if (el === root || (!el.scrollTop && !el.scrollLeft)) continue
      if (state.scroll.length < 40) state.scroll.push({ at: addressOf(el), top: Math.round(el.scrollTop), left: Math.round(el.scrollLeft) })
    }
    for (const el of Array.from(document.querySelectorAll('input,textarea,select'))) {
      if (state.fields.length >= MAX_FIELDS) break
      const info = fieldInfo(el)
      if (!recordableField(info)) continue
      const checkable = info.type === 'checkbox' || info.type === 'radio'
      state.fields.push({ at: addressOf(el), type: info.type, ...(checkable ? { checked: el.checked } : { value: el.value }) })
    }
    for (const el of Array.from(document.querySelectorAll('[role="tab"],.nav-tabs .nav-link,[data-bs-toggle="tab"],[data-bs-toggle="pill"]'))) {
      if (state.tabs.length >= MAX_TOGGLES) break
      const selected = el.getAttribute('aria-selected') === 'true' || (el.getAttribute('aria-selected') == null && el.classList.contains('active'))
      if (selected) state.tabs.push({ at: addressOf(el), label: labelOf(el) })
    }
    for (const el of Array.from(document.querySelectorAll('[aria-expanded]'))) {
      if (state.toggles.length >= MAX_TOGGLES) break
      if (el.getAttribute('role') === 'tab' || el.getAttribute('aria-haspopup') || el.closest('[role="menu"],[role="listbox"],[role="tablist"]')) continue
      state.toggles.push({ at: addressOf(el), label: labelOf(el), expanded: el.getAttribute('aria-expanded') === 'true' })
    }
    for (const el of Array.from(document.querySelectorAll('details'))) {
      if (state.toggles.length >= MAX_TOGGLES) break
      const summary = el.querySelector('summary')
      state.toggles.push({ at: addressOf(el), label: summary ? labelOf(summary) : '', expanded: el.open })
    }
    return state
  }

  // ---- writing it back --------------------------------------------------------------------------------------------------------------------------------------------------------------------

  const fire = (el, type) => el.dispatchEvent(new Event(type, { bubbles: true }))

  function setField(el, item) {
    if (isSensitiveField(fieldInfo(el)) || el.disabled || el.readOnly || el.type !== item.type) return true
    // A restored draft must also survive the next release. Synthetic events are not trusted.
    touched.add(el)
    if (typeof item.checked === 'boolean') {
      if (el.checked === item.checked) return true
      el.checked = item.checked
      fire(el, 'input'); fire(el, 'change')
      return true
    }
    if (el.value === item.value) return true
    // the native setter, so that a framework that wraps the property still sees the change
    const ctor = window[el.tagName === 'TEXTAREA' ? 'HTMLTextAreaElement' : el.tagName === 'SELECT' ? 'HTMLSelectElement' : 'HTMLInputElement']
    const setter = ctor && ctor.prototype ? Object.getOwnPropertyDescriptor(ctor.prototype, 'value') : null
    if (setter && setter.set) setter.set.call(el, item.value); else el.value = item.value
    fire(el, 'input'); fire(el, 'change')
    return true
  }

  /** Puts a snapshot back. The page may still be drawing: every pass applies what it can find and the rest waits for the next pass, for at most RESTORE_BUDGET. */
  function apply(raw, done) {
    const state = sanitize(raw)
    const pending = { scroll: state.scroll.slice(), fields: state.fields.slice(), tabs: state.tabs.slice(), toggles: state.toggles.slice() }
    let started = performance.now()
    let tick = 0
    let stopped = false
    const stop = () => { stopped = true }
    for (const type of ['pointerdown', 'keydown', 'wheel', 'touchstart']) window.addEventListener(type, stop, { capture: true, once: true, passive: true })
    const pass = () => {
      if (stopped) return finish('the person took over')
      pending.tabs = pending.tabs.filter((item) => {
        const el = find(item.at)
        if (!el || labelOf(el) !== item.label) return true
        if (el.getAttribute('aria-selected') !== 'true' && !el.classList.contains('active')) el.click()
        return false
      })
      pending.toggles = pending.toggles.filter((item) => {
        const el = find(item.at)
        if (!el) return true
        if (el.tagName === 'DETAILS') { if (el.open !== item.expanded) el.open = item.expanded; return false }
        if ((el.getAttribute('aria-expanded') === 'true') !== item.expanded) el.click()
        return false
      })
      pending.fields = pending.fields.filter((item) => { const el = find(item.at); return el ? !setField(el, item) : true })
      pending.scroll = pending.scroll.filter((item) => {
        const el = item.at && item.at.p && item.at.p.tag === '#root' ? document.scrollingElement : find(item.at)
        if (!el) return true
        // a box that is not tall enough yet (its content is still being drawn) is tried again
        if (el.scrollHeight - el.clientHeight + 2 < item.top) return true
        el.scrollTop = item.top
        el.scrollLeft = item.left
        return false
      })
      const left = pending.tabs.length + pending.toggles.length + pending.fields.length + pending.scroll.length
      if (!left) return finish('all applied')
      if (performance.now() - started > RESTORE_BUDGET) return finish(left + ' parts not found')
      tick = setTimeout(pass, RESTORE_STEP)
    }
    // a window made ahead of the click is hidden and put to sleep before the page is done: the time counts again from the moment it is shown
    const visible = () => { if (document.visibilityState === 'visible') { started = performance.now(); clearTimeout(tick); pass() } }
    document.addEventListener('visibilitychange', visible)
    const finish = (why) => {
      clearTimeout(tick)
      document.removeEventListener('visibilitychange', visible)
      for (const type of ['pointerdown', 'keydown', 'wheel', 'touchstart']) window.removeEventListener(type, stop, true)
      if (done) done(why)
    }
    pass()
  }

  // ---- the two roles ----------------------------------------------------------------------------------------------------------------------------------------------------------------------

  const isTop = window.parent === window
  let timer = 0
  let frameState = null
  let restoredFrame = null

  const parentOrigin = () => {
    for (const value of [document.referrer, location.ancestorOrigins && location.ancestorOrigins[0]]) {
      try {
        const url = new URL(value)
        const origin = url.origin === 'null' ? url.protocol + '//' + url.host : url.origin
        if (TRUSTED_PARENTS.has(origin)) return origin
      } catch (e) { /* not an address */ }
    }
    return ''
  }

  const frameElement = () => document.querySelector('iframe.sunshine-iframe') || document.querySelector('iframe')
  const frameOrigin = () => { try { return new URL(frameElement().src, location.href).origin } catch (e) { return '' } }

  function snapshot() {
    const here = collect()
    if (isTop) {
      const nav = typeof window.__panelNavState === 'function' ? window.__panelNavState() : null
      return { v: 1, nav, shell: here, frame: frameState }
    }
    return here
  }

  function save() {
    timer = 0
    let state
    try { state = snapshot() } catch (e) { return }
    if (isTop) {
      try { return window.__TAURI_INTERNALS__.invoke('panel_state_save', { state: JSON.stringify(state) }).catch(() => {}) } catch (e) { /* the native side keeps the last one */ }
    } else {
      const origin = parentOrigin()
      if (origin) { try { window.parent.postMessage({ type: 'panel-view-state', state }, origin) } catch (e) { /* no parent */ } }
    }
  }

  function schedule() {
    if (timer) return
    timer = setTimeout(save, SAVE_DELAY)
  }

  function flush() {
    if (timer) { clearTimeout(timer); timer = 0 }
    return save()
  }

  function start() {
    document.addEventListener('scroll', (event) => {
      const target = event.target
      if (target && target.nodeType === 1) scrolled.set(target, 1)
      schedule()
    }, { capture: true, passive: true })
    for (const type of ['input', 'change']) document.addEventListener(type, (event) => { if (event.isTrusted && event.target && event.target.nodeType === 1) touched.add(event.target); schedule() }, true)
    document.addEventListener('click', () => setTimeout(schedule, 60), true)
    document.addEventListener('visibilitychange', () => { if (document.visibilityState === 'hidden') flush() })
    window.addEventListener('pagehide', flush)
    if (isTop) {
      window.addEventListener('message', (event) => {
        const frame = frameElement()
        if (!frame || event.source !== frame.contentWindow || event.origin !== frameOrigin()) return
        const data = event.data
        if (!data || typeof data !== 'object') return
        if (data.type === 'panel-view-state') { frameState = sanitize(data.state); schedule() }
        else if (data.type === 'panel-view-ready') {
          // the page in the frame says it is ready: if it is the page the snapshot was taken on, it gets its state back (once)
          const saved = restoredFrame
          if (saved && samePage(saved, { path: data.path })) {
            restoredFrame = null
            try { frame.contentWindow.postMessage({ type: 'panel-view-restore', state: saved }, event.origin) } catch (e) { /* the frame went away */ }
          }
        }
      })
      const restore = window.__PANEL_RESTORE__
      if (restore && typeof restore === 'object') {
        restoredFrame = restore.frame ? sanitize(restore.frame) : null
        if (restore.frame) frameState = restoredFrame
        const begin = () => apply(restore.shell, () => {})
        if (document.readyState === 'complete') setTimeout(begin, 0); else window.addEventListener('load', () => setTimeout(begin, 0), { once: true })
      }
    } else {
      window.addEventListener('message', (event) => {
        if (event.source !== window.parent || event.origin !== parentOrigin()) return
        const data = event.data
        if (data && data.type === 'panel-view-restore') apply(data.state, () => {})
      })
      const hello = () => {
        const origin = parentOrigin()
        if (origin) { try { window.parent.postMessage({ type: 'panel-view-ready', path: location.pathname + location.search + location.hash }, origin) } catch (e) { /* no parent */ } }
      }
      if (document.readyState === 'complete') setTimeout(hello, 0); else window.addEventListener('load', () => setTimeout(hello, 0), { once: true })
    }
  }

  window.__panelViewState = { keyParts, selectorOf, isSensitiveField, recordableField, samePage, sanitize, collect, apply, flush }
  try { start() } catch (e) { /* a page without these APIs keeps working without the snapshot */ }
})()
