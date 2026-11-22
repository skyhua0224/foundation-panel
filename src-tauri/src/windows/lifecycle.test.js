import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import vm from 'node:vm'

const source = readFileSync(new URL('./lifecycle.js', import.meta.url), 'utf8')

function page(generation, framed = false) {
  const calls = [], timers = [], events = [], styles = []
  const head = {
    appendChild(style) { styles.push(style); style.parentNode = this },
    removeChild(style) { styles.splice(styles.indexOf(style), 1); style.parentNode = null },
  }
  const document = { head, documentElement: head,
    createElement: () => ({ parentNode: null }), dispatchEvent: event => events.push(event.type) }
  const window = { __TAURI_INTERNALS__: { invoke: (name, args) => { calls.push({ name, args }); return Promise.reject(new Error('not allowed by ACL')) } } }
  window.top = framed ? {} : window
  vm.runInNewContext(source.replaceAll('__LIFECYCLE_GENERATION__', String(generation)), {
    window, document, Event: class { constructor(type) { this.type = type } },
    console: { log() {} }, setInterval: (callback, delay) => timers.push({ callback, delay }),
  })
  return { window, document, calls, timers, events, styles }
}

test('initial and periodic heartbeats identify the window lifetime', () => {
  const old = page(7), current = page(8)
  old.timers[0].callback()
  current.timers[0].callback()
  assert.equal(old.timers[0].delay, 10000)
  for (const call of old.calls) {
    assert.equal(call.name, 'webview_heartbeat')
    assert.equal(call.args.generation, 7)
  }
  for (const call of current.calls) assert.equal(call.args.generation, 8)
})

test('visibility notifications pause and resume page content only on transitions', () => {
  const view = page(1)
  view.window.__setWebviewVisibility(false)
  view.window.__setWebviewVisibility(false)
  assert.equal(view.document.hidden, true)
  assert.equal(view.document.visibilityState, 'hidden')
  assert.equal(view.styles.length, 1)
  assert.match(view.styles[0].textContent, /animation-play-state: paused/)
  assert.deepEqual(view.events, ['visibilitychange'])
  view.window.__setWebviewVisibility(true)
  assert.equal(view.document.hidden, false)
  assert.equal(view.styles.length, 0)
  assert.deepEqual(view.events, ['visibilitychange', 'visibilitychange'])
})

test('a frame of another origin does not beat, and a rejected beat is not an unhandled rejection', async () => {
  const frame = page(3, true)
  frame.timers[0].callback()
  assert.equal(frame.calls.length, 0, 'the Web UI frame never calls the command')
  const top = page(3)
  const rejections = []
  process.on('unhandledRejection', (e) => rejections.push(e))
  top.timers[0].callback()
  await new Promise((r) => setTimeout(r, 20))
  process.removeAllListeners('unhandledRejection')
  assert.ok(top.calls.length >= 2, 'the window itself still beats')
  assert.equal(rejections.length, 0)
})

// A page whose engine reports its own visibility (a window covered by other windows) next to the panel's.
function engineLoaded(initialEngineHidden) {
  const state = { engineHidden: initialEngineHidden }
  const styles = []
  const listeners = []
  const events = []
  const head = {
    appendChild(style) { styles.push(style); style.parentNode = this },
    removeChild(style) { styles.splice(styles.indexOf(style), 1); style.parentNode = null },
  }
  const document = {
    head, documentElement: head,
    createElement: () => ({ parentNode: null }),
    addEventListener: (type, fn) => type === 'visibilitychange' && listeners.push(fn),
    dispatchEvent: (event) => { events.push(event.type); listeners.slice().forEach((fn) => fn(event)) },
  }
  class Document {}
  Object.defineProperty(Document.prototype, 'visibilityState', { get() { return state.engineHidden ? 'hidden' : 'visible' }, configurable: true })
  Object.setPrototypeOf(document, Document.prototype)
  const window = { __TAURI_INTERNALS__: undefined }
  window.top = window
  vm.runInNewContext(source.replaceAll('__LIFECYCLE_GENERATION__', '1'), {
    window, document, Document, Event: class { constructor(type) { this.type = type } },
    console: { log() {} }, setInterval: () => 0,
  })
  // the engine tells its listeners (the page's and the script's own) when its state changes
  const engineChanges = (hidden) => { state.engineHidden = hidden; listeners.slice().forEach((fn) => fn({ type: 'visibilitychange' })) }
  return { window, document, styles, events, engineChanges }
}

test('a covered window is hidden for the page and its animations pause, shown again when uncovered', () => {
  const view = engineLoaded(false)
  assert.equal(view.document.hidden, false)
  view.engineChanges(true)
  assert.equal(view.document.hidden, true)
  assert.equal(view.document.visibilityState, 'hidden')
  assert.equal(view.styles.length, 1, 'the animations pause for a covered window too')
  view.engineChanges(false)
  assert.equal(view.document.hidden, false)
  assert.equal(view.styles.length, 0)
})

test('the panel and the engine are two sources of one state', () => {
  const view = engineLoaded(false)
  view.window.__setWebviewVisibility(false)
  assert.equal(view.styles.length, 1)
  // the engine also reports hidden: still hidden, still one pause style
  view.engineChanges(true)
  assert.equal(view.styles.length, 1)
  // the panel shows the window but the engine still says covered: still hidden
  view.window.__setWebviewVisibility(true)
  assert.equal(view.document.hidden, true)
  assert.equal(view.styles.length, 1)
  view.engineChanges(false)
  assert.equal(view.document.hidden, false)
  assert.equal(view.styles.length, 0)
})

test('a frame that starts covered pauses at once on its first engine event and not before', () => {
  const view = engineLoaded(true)
  assert.equal(view.document.hidden, true)
  view.engineChanges(true)
  assert.equal(view.styles.length, 1)
})
