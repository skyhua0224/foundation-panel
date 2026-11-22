// The recorder of the page state (src-tauri/inject-viewstate.js) and the router's use of a snapshot (useRouter.js restoredStack).
import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import vm from 'node:vm'
import { restoredStack } from './composables/restoredStack.js'

const SOURCE = readFileSync(new URL('../../src-tauri/inject-viewstate.js', import.meta.url), 'utf8')

class FakeEvent { constructor(type) { this.type = type } }

/** A document that is only what the recorder asks of it: elements registered under the selectors it will use. */
function world(registry = {}, options = {}) {
  const listeners = []
  const ctx = {
    document: {
      readyState: 'complete', referrer: '', visibilityState: 'visible',
      scrollingElement: options.root || null,
      addEventListener() {}, removeEventListener() {},
      querySelectorAll: (selector) => registry[selector] || [],
    },
    location: { pathname: '/config', search: '', hash: '#tab=input', ancestorOrigins: [] },
    performance: { now: () => Date.now() },
    setTimeout, clearTimeout, Event: FakeEvent, console, Math, JSON, Array, Object, String, Number, Set, Map, WeakSet, URL,
  }
  ctx.window = ctx
  ctx.parent = ctx
  ctx.addEventListener = (type, fn, opts) => listeners.push({ type, fn, opts })
  ctx.removeEventListener = (type, fn) => { const i = listeners.findIndex((l) => l.type === type && l.fn === fn); if (i >= 0) listeners.splice(i, 1) }
  vm.createContext(ctx)
  vm.runInContext(SOURCE, ctx)
  return { ctx, api: ctx.__panelViewState, listeners }
}

function element(tag, attrs = {}, extra = {}) {
  const log = []
  const el = {
    tagName: tag.toUpperCase(), id: attrs.id || '', value: attrs.value ?? '', checked: !!attrs.checked, type: attrs.type || 'text', disabled: false, readOnly: false,
    classList: { contains: (c) => (attrs.classes || []).includes(c), [Symbol.iterator]: function* () { yield* (attrs.classes || []) } },
    getAttribute: (name) => (name in attrs ? attrs[name] : null),
    dispatchEvent: (event) => { log.push(event.type); return true },
    click: () => log.push('click'),
    labels: null, textContent: attrs.text || '', isConnected: true,
    scrollTop: 0, scrollLeft: 0, scrollHeight: 0, clientHeight: 0,
    ...extra,
  }
  el.log = log
  return el
}

const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms))

test('an element is found again by its id, its name or its stable classes, never by a state class', () => {
  const { api } = world()
  const parts = api.keyParts({ tag: 'DIV', id: '', classes: ['m-scene__pane', 'is-open', 'v-abc', 'active', 'pane-12345', 'second', 'third'] })
  assert.deepEqual([...parts.classes], ['m-scene__pane', 'second'])
  assert.equal(api.selectorOf(parts), 'div.m-scene__pane.second')
  assert.equal(api.selectorOf(api.keyParts({ tag: 'input', id: 'bitrate' })), '#bitrate')
  assert.equal(api.selectorOf(api.keyParts({ tag: 'input', id: 'field-123456', name: 'fps' })), 'input[name="fps"]', 'an id with a generated number is not a key')
  assert.equal(api.selectorOf(api.keyParts({ tag: 'div', key: 'detail' })), '[data-scroll-key="detail"]')
})

test('secrets are never recorded', () => {
  const { api } = world()
  for (const info of [
    { type: 'password' }, { type: 'hidden' }, { type: 'file' },
    { type: 'text', name: 'api_key' }, { type: 'text', label: 'Pairing PIN' }, { type: 'text', autocomplete: 'one-time-code' },
    { type: 'text', placeholder: 'Access token' }, { type: 'text', id: 'client-secret' }, { type: 'text', ariaLabel: 'Credit card number' },
  ]) assert.equal(api.isSensitiveField(info), true, JSON.stringify(info))
  for (const info of [{ type: 'text', name: 'bitrate' }, { type: 'text', label: 'Keyboard layout' }, { type: 'text', name: 'spinner' }, { type: 'number', id: 'fps' }, { type: 'text', label: 'Pinned apps' }]) {
    assert.equal(api.isSensitiveField(info), false, JSON.stringify(info))
  }
})

test('only what the person typed is a draft, and an absurd value is not', () => {
  const { api } = world()
  assert.equal(api.recordableField({ type: 'text', name: 'name', value: 'abc', touched: true }), true)
  assert.equal(api.recordableField({ type: 'text', name: 'name', value: 'abc', touched: false }), false, 'a value the page set itself is not a draft')
  assert.equal(api.recordableField({ type: 'text', name: 'name', value: 'x'.repeat(4001), touched: true }), false)
  assert.equal(api.recordableField({ type: 'password', value: 'abc', touched: true }), false)
  assert.equal(api.recordableField({ type: 'submit', value: 'Save', touched: true }), false)
})

test('a snapshot from outside is data: only the known shape is taken, and it is bounded', () => {
  const { api } = world()
  const clean = api.sanitize({
    scroll: [{ at: { p: { tag: 'div' }, i: 0 }, top: 120.7, left: -5 }, ...Array.from({ length: 80 }, () => ({ at: {}, top: 1, left: 1 }))],
    fields: [{ at: {}, type: 'text', value: 'y'.repeat(9000) }, { at: {}, type: 'text', value: 42 }, { at: {}, type: 'checkbox', checked: true }],
    tabs: [{ at: {}, label: 'l'.repeat(300) }], toggles: [{ at: {}, label: 'More', expanded: 1 }], path: '/config#a',
    evil: '<script>', __proto__: { polluted: true },
  })
  assert.equal(clean.scroll.length, 40)
  assert.deepEqual([clean.scroll[0].top, clean.scroll[0].left], [121, 0])
  assert.equal(clean.fields.length, 2, 'a number is not a value')
  assert.equal(clean.fields[0].value.length, 4000)
  assert.equal(clean.tabs[0].label.length, 80)
  assert.equal(clean.toggles[0].expanded, false, 'only true is true')
  assert.equal(clean.path, '/config#a')
  assert.equal('evil' in clean, false)
  assert.deepEqual(JSON.parse(JSON.stringify(api.sanitize('nonsense'))), { scroll: [], fields: [], tabs: [], toggles: [] })
})

test('the state of a frame page goes back to that page only', () => {
  const { api } = world()
  assert.equal(api.samePage({ path: '/config#tab=input' }, { path: '/config#tab=input' }), true)
  assert.equal(api.samePage({ path: '/config#tab=input' }, { path: '/apps' }), false)
  assert.equal(api.samePage(null, { path: '/' }), false)
})

test('a restore puts a draft back as the person typed it and tells the page, but leaves a field that already holds it alone', async () => {
  const typed = element('input', { id: 'name', value: '' })
  const same = element('input', { id: 'port', value: '47989' })
  const secret = element('input', { id: 'api-token', value: '' })
  const { api } = world({ '#name': [typed], '#port': [same], '#api-token': [secret] })
  const done = new Promise((resolve) => api.apply({
    fields: [
      { at: { p: { tag: 'input', id: 'name' }, i: 0 }, type: 'text', value: 'my host' },
      { at: { p: { tag: 'input', id: 'port' }, i: 0 }, type: 'text', value: '47989' },
      { at: { p: { tag: 'input', id: 'api-token' }, i: 0 }, type: 'text', value: 'leaked' },
    ],
  }, resolve))
  assert.equal(await done, 'all applied')
  assert.equal(typed.value, 'my host')
  assert.deepEqual(typed.log, ['input', 'change'], 'the page hears it, as it hears typing')
  assert.deepEqual(same.log, [], 'a field that holds the value is not touched: the page does not see a change')
  assert.equal(secret.value, '', 'a secret is never written, even if a snapshot carried one')
})

test('a restored draft remains eligible for a second release', async () => {
  const input = element('input', { id: 'draft' })
  const { api } = world({ '#draft': [input], 'input,textarea,select': [input] })
  await new Promise((resolve) => api.apply({
    fields: [{ at: { p: { tag: 'input', id: 'draft' }, i: 0 }, type: 'text', value: 'carry me twice' }],
  }, resolve))
  assert.equal(api.collect().fields[0].value, 'carry me twice')
})

test('a scroll is applied once the box is tall enough, and a missing part ends the restore after its budget, not before', async () => {
  const pane = element('div', { classes: ['m-scene__pane'] }, { scrollHeight: 500, clientHeight: 400 })
  const { api, ctx } = world({ 'div.m-scene__pane': [pane] })
  const done = new Promise((resolve) => api.apply({ scroll: [{ at: { p: { tag: 'div', classes: ['m-scene__pane'] }, i: 0 }, top: 900, left: 0 }] }, resolve))
  await wait(150)
  assert.equal(pane.scrollTop, 0, 'the content is not tall enough yet: nothing is forced')
  pane.scrollHeight = 1600
  assert.equal(await done, 'all applied')
  assert.equal(pane.scrollTop, 900)
  void ctx
})

test('a tab and a section come back; one that is already right is not clicked', async () => {
  const wrong = element('button', { role: 'tab', 'aria-selected': 'false', text: 'Keyboard' })
  const right = element('button', { role: 'tab', 'aria-selected': 'true', text: 'Mouse' })
  const closed = element('button', { 'aria-expanded': 'false', text: 'Advanced' })
  const open = element('button', { 'aria-expanded': 'true', text: 'Network' })
  const { api } = world({ '#t1': [wrong], '#t2': [right], '#s1': [closed], '#s2': [open] })
  const done = new Promise((resolve) => api.apply({
    tabs: [{ at: { p: { tag: 'button', id: 't1' }, i: 0 }, label: 'Keyboard' }, { at: { p: { tag: 'button', id: 't2' }, i: 0 }, label: 'Mouse' }],
    toggles: [{ at: { p: { tag: 'button', id: 's1' }, i: 0 }, label: 'Advanced', expanded: true }, { at: { p: { tag: 'button', id: 's2' }, i: 0 }, label: 'Network', expanded: true }],
  }, resolve))
  assert.equal(await done, 'all applied')
  assert.deepEqual(wrong.log, ['click'])
  assert.deepEqual(right.log, [])
  assert.deepEqual(closed.log, ['click'])
  assert.deepEqual(open.log, [])
})

test('the person taking over ends a restore: nothing is applied after their first input', async () => {
  const field = element('input', { id: 'late', value: '' })
  const { api, listeners } = world({})
  const done = new Promise((resolve) => api.apply({ fields: [{ at: { p: { tag: 'input', id: 'late' }, i: 0 }, type: 'text', value: 'x' }] }, resolve))
  listeners.find((l) => l.type === 'pointerdown').fn()
  assert.equal(await done, 'the person took over')
  assert.equal(field.value, '')
})

test('the router builds its stack from the snapshot', () => {
  const router = readFileSync(new URL('./composables/useRouter.js', import.meta.url), 'utf8')
  assert.match(router, /initial: restoredStack\(globalThis\.__PANEL_RESTORE__, routes\)/)
  assert.match(router, /globalThis\.__panelNavState = /)
})

test('the router takes a saved stack only if every destination is one it knows', () => {
  const known = { home: {}, 'web-config': {}, about: {} }
  assert.deepEqual(restoredStack({ nav: [{ id: 'home' }, { id: 'web-config', params: { a: 1 } }] }, known), [{ id: 'home' }, { id: 'web-config', params: { a: 1 } }])
  assert.deepEqual(restoredStack({ nav: [{ id: 'about', root: true }] }, known), [{ id: 'about', root: true }])
  assert.equal(restoredStack({ nav: [{ id: 'gone-page' }] }, known), null)
  assert.equal(restoredStack({ nav: [{ id: 'constructor' }] }, known), null, 'a name of the object prototype is not a destination')
  assert.equal(restoredStack({ nav: [] }, known), null)
  assert.equal(restoredStack({ nav: Array.from({ length: 9 }, () => ({ id: 'home' })) }, known), null)
  assert.equal(restoredStack(null, known), null)
  assert.equal(restoredStack({ nav: 'home' }, known), null)
})
