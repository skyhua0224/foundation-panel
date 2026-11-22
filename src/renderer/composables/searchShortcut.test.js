import assert from 'node:assert/strict'
import test from 'node:test'
import { createSearchShortcutHandler, installSearchShortcut, isComposingKey, isEditableTarget, shouldIgnoreSlash } from './searchShortcut.js'

const el = (tagName, extra = {}) => ({ nodeType: 1, tagName, parentElement: null, getAttribute: () => null, ...extra })
const key = (target, props = {}, path) => ({ key: '/', ctrlKey: false, metaKey: false, altKey: false, shiftKey: false, isComposing: false, keyCode: 191, target, composedPath: () => path || [target], ...props })
const body = el('BODY')
const doc = { activeElement: body }

// a search that records what the shortcut handler passes on, like useUnifiedSearch().onShortcut
// (it handles only the keys the real one handles: Ctrl/Cmd+K and F, a plain "/" and Esc)
const fakeSearch = () => {
  const seen = []
  return {
    seen,
    onShortcut: (e) => {
      const mod = e.ctrlKey || e.metaKey
      const handled = (mod && !e.altKey && !e.shiftKey && /^[kKfF]$/.test(e.key)) || (e.key === '/' && !mod && !e.altKey) || e.key === 'Escape'
      if (handled) seen.push(e.key)
      return handled
    },
  }
}

test('"/" typed into input, textarea, select and contenteditable stays a character', () => {
  for (const target of [el('INPUT'), el('TEXTAREA'), el('SELECT'), el('DIV', { isContentEditable: true })])
    assert.equal(shouldIgnoreSlash(key(target), doc), true, target.tagName)
})

test('"/" in a Material text field is guarded although the event target is the shadow host', () => {
  const host = el('MD-OUTLINED-TEXT-FIELD')
  const inner = el('INPUT')
  // outside the shadow root the target is the host; the composed path still holds the inner input
  assert.equal(isEditableTarget(key(host, {}, [inner, host, body]), doc), true)
  assert.equal(isEditableTarget(key(host, {}, [host, body]), doc), true)
  assert.equal(isEditableTarget(key(body, {}, [body]), { activeElement: { ...host, shadowRoot: { activeElement: inner } } }), true)
})

test('role=textbox / combobox and a child of a contenteditable are fields', () => {
  assert.equal(isEditableTarget(key(el('DIV', { getAttribute: (n) => (n === 'role' ? 'textbox' : null) })), doc), true)
  assert.equal(isEditableTarget(key(el('DIV', { getAttribute: (n) => (n === 'role' ? 'combobox' : null) })), doc), true)
  const editor = el('DIV', { isContentEditable: true })
  assert.equal(isEditableTarget(key(el('SPAN', { parentElement: editor }), {}, [])), true)
})

test('"/" during IME composition is guarded even outside a field', () => {
  assert.equal(shouldIgnoreSlash(key(body, { isComposing: true }), doc), true)
  assert.equal(shouldIgnoreSlash(key(body, { keyCode: 229 }), doc), true)
  assert.equal(isComposingKey(key(body, { key: 'Process' })), true)
  assert.equal(isComposingKey(key(body)), false)
})

test('"/" on the page (no field, no composition) still opens the search', () => {
  const search = fakeSearch()
  const handle = createSearchShortcutHandler(search, doc)
  assert.equal(handle(key(body)), true)
  assert.deepEqual(search.seen, ['/'])
})

test('"/" in a field never reaches the search; typing https:// keeps all characters', () => {
  const search = fakeSearch()
  const handle = createSearchShortcutHandler(search, doc)
  const host = el('MD-OUTLINED-TEXT-FIELD')
  const path = [el('INPUT'), host, body]
  let typed = ''
  for (const ch of 'https://api.example.com:8443/v1') {
    const e = key(host, { key: ch }, path)
    if (!handle(e)) typed += ch // the key was left to the field
  }
  assert.equal(typed, 'https://api.example.com:8443/v1')
  assert.deepEqual(search.seen.filter((k) => k === '/'), [])
})

test('pasted text that arrives as key events keeps its "//"', () => {
  const search = fakeSearch()
  const handle = createSearchShortcutHandler(search, doc)
  const input = el('INPUT')
  let typed = ''
  for (const ch of 'http://192.168.1.10:11434/v1/') if (!handle(key(input, { key: ch }))) typed += ch
  assert.equal(typed, 'http://192.168.1.10:11434/v1/')
})

test('Ctrl K and Ctrl F keep working in a field and while composing', () => {
  const search = fakeSearch()
  const handle = createSearchShortcutHandler(search, doc)
  const input = el('INPUT')
  assert.equal(handle(key(input, { key: 'k', ctrlKey: true })), true)
  assert.equal(handle(key(input, { key: 'K', metaKey: true })), true)
  assert.equal(handle(key(input, { key: 'f', ctrlKey: true, isComposing: true })), true)
  assert.deepEqual(search.seen, ['k', 'K', 'f'])
})

test('Ctrl+/ and Alt+/ are not the plain slash; the guard leaves them to the search', () => {
  const search = fakeSearch()
  const handle = createSearchShortcutHandler(search, doc)
  assert.equal(handle(key(el('INPUT'), { ctrlKey: true })), false)
  assert.equal(handle(key(el('INPUT'), { altKey: true })), false)
  assert.deepEqual(search.seen, []) // not guarded here: the search itself decides, and it does not take them
})

test('Esc still reaches the search from a field', () => {
  const search = fakeSearch()
  createSearchShortcutHandler(search, doc)(key(el('INPUT'), { key: 'Escape' }))
  assert.deepEqual(search.seen, ['Escape'])
})

test('installSearchShortcut listens in the capture phase and removes itself', () => {
  const calls = []
  const fakeDoc = { addEventListener: (...a) => calls.push(['add', ...a]), removeEventListener: (...a) => calls.push(['remove', ...a]) }
  const off = installSearchShortcut(fakeSearch(), fakeDoc)
  off()
  assert.equal(calls[0][0], 'add'); assert.equal(calls[0][1], 'keydown'); assert.equal(calls[0][3], true)
  assert.equal(calls[1][0], 'remove'); assert.equal(calls[1][2], calls[0][2]); assert.equal(calls[1][3], true)
})

// the wiring: the shared search must not also listen by itself, and the address field must hand its text to the model unchanged
import { readFileSync } from 'node:fs'
const source = (rel) => readFileSync(new URL(rel, import.meta.url), 'utf8')

test('the sidebar creates the search without its own shortcuts and installs the guarded listener', () => {
  const sidebar = source('../components/SidebarMenu.vue')
  assert.match(sidebar, /useUnifiedSearch\(\{[^\n]*shortcuts: false \}\)/)
  assert.match(sidebar, /installSearchShortcut\(uni\)/)
})

test('the API address field is a plain model binding: no sanitizing step can cut a pasted URL at "//"', () => {
  const assistant = source('../components/AiAssistant.vue')
  assert.match(assistant, /<MFieldItem v-model="config\.apiBase"/)
  const client = source('./aiClient.js')
  // the only edit made to the address is dropping trailing slashes, which keeps scheme, host, port and path
  const trim = (base) => base.replace(/\/+$/, '')
  assert.ok(client.includes("apiBase.replace(/\\/+$/, '')"))
  for (const url of ['https://api.example.com/v1', 'http://127.0.0.1:11434/v1', 'https://host.example:8443/a//b/c'])
    assert.equal(trim(url), url)
  assert.equal(trim('https://api.example.com/v1///'), 'https://api.example.com/v1')
})
