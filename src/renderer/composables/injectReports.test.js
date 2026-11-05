import assert from 'node:assert/strict'
import fs from 'node:fs'
import test from 'node:test'
import vm from 'node:vm'
import { normalizeFrameReport } from './frameReport.js'

// The script the proxy puts into every Web UI page, run against a page that is not there: what it posts to the panel when the page fails, is slow or is blocked.
const code = fs.readFileSync(new URL('../../../src-tauri/inject-script.js', import.meta.url), 'utf8')

function page({ framed = true, root = null } = {}) {
  let clock = 0
  const posts = []
  const listeners = {}
  const observers = []
  class PerformanceObserver {
    constructor(callback) { this.callback = callback; observers.push(this) }
    observe(options) { this.options = options }
    disconnect() {}
  }
  const mutations = []
  class MutationObserver { constructor(callback) { this.callback = callback; mutations.push(this) } observe() {} }
  const sandbox = {
    URL, setTimeout, clearTimeout, console,
    performance: { now: () => clock },
    PerformanceObserver, MutationObserver,
    document: { referrer: 'http://tauri.localhost/', readyState: 'complete', addEventListener() {}, body: {}, documentElement: root || { dataset: {}, setAttribute() {}, getAttribute: () => null, style: {} }, querySelectorAll: () => [] },
    location: { pathname: '/pin', search: '', hash: '', href: 'http://127.0.0.1:48081/pin', ancestorOrigins: ['http://tauri.localhost'] },
    localStorage: { setItem() {}, getItem: () => null },
    addEventListener: (type, fn) => { (listeners[type] ||= []).push(fn) },
    fetch: () => { clock += 2500; return Promise.resolve({}) },
  }
  sandbox.window = sandbox
  sandbox.postMessage = () => {}
  sandbox.parent = framed ? { postMessage: (message) => posts.push(message) } : sandbox
  vm.runInNewContext(code, sandbox)
  const reports = () => posts.filter((p) => p.type === 'panel-frame-report')
  return { sandbox, posts, reports, listeners, observers, mutations, advance: (ms) => { clock += ms } }
}

test('an error of the page is posted with its message and where it was', () => {
  const p = page()
  p.listeners.error[0]({ message: 'Cannot read properties of null (reading \'uuid\')', filename: 'http://127.0.0.1:48081/assets/usePin-7ilq7nu1.js', lineno: 1, colno: 1192, error: { stack: 'TypeError: x\n    at w (usePin.js:1:1192)\n    at async refresh (Pin.js:1:1)' } })
  const [r] = p.reports()
  assert.equal(r.kind, 'error'); assert.equal(r.page, '/pin'); assert.match(r.name, /reading 'uuid'/); assert.match(r.detail, /^usePin-7ilq7nu1\.js:1:1192\s+at w/)
  assert.deepEqual(normalizeFrameReport(r), { kind: 'frame-error', name: `/pin ${r.name}`, ms: 0, detail: r.detail.replace(/\s+/g, ' ') }, 'the panel accepts what the page sends')
})
test('a rejection nobody caught is posted, a ResizeObserver notice is not', () => {
  const p = page()
  p.listeners.unhandledrejection[0]({ reason: new Error('boom') })
  p.listeners.error[0]({ message: 'ResizeObserver loop completed with undelivered notifications.' })
  assert.deepEqual(p.reports().map((r) => r.name), ['unhandledrejection: boom'])
})
test('a long frame is posted with its script, a short one is not', () => {
  const p = page()
  const loaf = p.observers.find((o) => o.options?.type === 'long-animation-frame')
  loaf.callback({ getEntries: () => [{ duration: 120, entryType: 'long-animation-frame' }, { duration: 842, blockingDuration: 790, entryType: 'long-animation-frame', scripts: [{ invokerType: 'user-callback', invoker: 'Worker.onmessage', duration: 700, sourceURL: 'http://x/assets/vue-abc.js', sourceCharPosition: 214 }] }] })
  const reports = p.reports()
  assert.equal(reports.length, 1); assert.equal(reports[0].kind, 'frame'); assert.equal(reports[0].ms, 842); assert.match(reports[0].detail, /Worker\.onmessage:700ms@vue-abc\.js:214/)
})
test('a slow call is posted by its path, the pairing long poll is not', async () => {
  const p = page()
  await p.sandbox.fetch('http://127.0.0.1:48081/api/clients/list?token=abc')
  await p.sandbox.fetch('/api/pairing/pending?wait=25&since=3')
  const reports = p.reports()
  assert.equal(reports.length, 1); assert.equal(reports[0].kind, 'fetch'); assert.equal(reports[0].name, '/api/clients/list'); assert.ok(!JSON.stringify(reports[0]).includes('abc'), 'the query stays in the page')
})
test('a page that fails in a loop posts at most 20 reports a minute', () => {
  const p = page()
  for (let i = 0; i < 100; i += 1) p.listeners.error[0]({ message: `e${i}` })
  assert.equal(p.reports().length, 20)
})
test('the first paint carries its time, and a page that is not in a frame posts no reports', () => {
  const p = page()
  p.advance(640)
  const paint = p.observers.find((o) => o.options?.type === 'paint')
  paint.callback({ getEntries: () => [{ name: 'first-contentful-paint' }] }, { disconnect() {} })
  assert.deepEqual(p.posts.filter((m) => m.type === 'panel-painted').map((m) => m.fcp), [640])
  const alone = page({ framed: false })
  assert.equal(alone.listeners.error, undefined, 'no handler in a page that is its own window')
})

function rootElement(attrs = {}, style = {}) {
  return { dataset: {}, setAttribute(k, v) { attrs[k] = v }, getAttribute: (k) => (k in attrs ? attrs[k] : null), style, attrs }
}
test('the root of a framed page follows the theme of the panel: no stale dark scheme, no background of its own', () => {
  // the Web page wrote the system's scheme in its <head> (dark); the panel is light
  const root = rootElement({ 'data-bs-theme': 'light' }, { colorScheme: 'dark', backgroundColor: 'rgb(20, 18, 24)' })
  page({ root })
  assert.equal(root.style.colorScheme, 'light'); assert.equal(root.style.backgroundColor, '')
})
test('a theme change while the page is open reaches its colour scheme', () => {
  const root = rootElement({ 'data-bs-theme': 'dark' }, { colorScheme: 'dark', backgroundColor: 'rgb(20, 18, 24)' })
  const p = page({ root })
  assert.equal(root.style.colorScheme, 'dark')
  root.attrs['data-bs-theme'] = 'light'
  p.mutations.find((m) => m.callback.name === 'apply').callback([])
  assert.equal(root.style.colorScheme, 'light')
  root.style.backgroundColor = 'rgb(1, 2, 3)'  // a script of the page writes one again
  p.mutations.find((m) => m.callback.name === 'apply').callback([])
  assert.equal(root.style.backgroundColor, '')
})
test('a page without a theme attribute, or one that is not in a frame, keeps what it wrote', () => {
  const none = rootElement({}, { colorScheme: 'dark', backgroundColor: 'rgb(20, 18, 24)' })
  page({ root: none })
  assert.equal(none.style.colorScheme, 'dark'); assert.equal(none.style.backgroundColor, 'rgb(20, 18, 24)')
  const top = rootElement({ 'data-bs-theme': 'light' }, { colorScheme: 'dark', backgroundColor: 'rgb(20, 18, 24)' })
  page({ framed: false, root: top })
  assert.equal(top.style.colorScheme, 'dark')
})
