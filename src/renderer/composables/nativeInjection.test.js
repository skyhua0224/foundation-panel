import assert from 'node:assert/strict'
import test from 'node:test'
import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'

const script = readFileSync(new URL('../../../src-tauri/inject-script.js', import.meta.url), 'utf8')

test('remote grants cover the injected WebUI integration without unrelated host commands', () => {
  const permissions = JSON.parse(readFileSync(new URL('../../../src-tauri/webui-permissions.json', import.meta.url), 'utf8'))
  for (const [, command] of script.matchAll(/\binvoke\('([^']+)'/g)) {
    assert.ok(permissions.includes(`allow-${command.replaceAll('_', '-')}`), command)
  }
  for (const command of ['launch-game', 'capture-screenshot', 'install-update', 'restart-as-admin']) {
    assert.ok(!permissions.includes(`allow-${command}`), command)
  }
})

function createHarness({ pathname = '/config', hash = '', embedded = true } = {}) {
  const messages = []
  const listeners = {}
  const tabs = []
  const location = { pathname, search: '', hash, ancestorOrigins: ['http://tauri.localhost'] }
  const document = {
    referrer: 'http://tauri.localhost/',
    readyState: 'complete',
    documentElement: { dataset: {}, getAttribute: () => null, style: {} },
    body: {},
    querySelectorAll: () => tabs,
    querySelector: () => logoutButton,
  }
  let logoutButton = null
  let mutations
  const historyCalls = []
  const history = {
    state: { from: 'panel-settings' },
    replaceState: (...args) => {
      historyCalls.push(args)
      location.hash = ''
    },
  }
  const window = {
    addEventListener: (type, handler) => {
      listeners[type] = handler
    },
  }
  window.parent = embedded ? { postMessage: (...args) => messages.push(args) } : window
  window.postMessage = (...args) => messages.push(args)
  runInNewContext(script, {
    window,
    document,
    location,
    history,
    URL,
    MutationObserver: class {
      constructor(callback) {
        mutations = callback
      }
      observe() {}
    },
  })
  const sendContext = (overrides = {}) =>
    listeners.message({
      source: window.parent,
      origin: 'http://tauri.localhost',
      data: { type: 'native-navigation-context', source: 'sunshine-control-panel', unified: true },
      ...overrides,
    })
  return {
    document,
    messages,
    tabs,
    history,
    historyCalls,
    location,
    sendContext,
    mutate: () => mutations?.(),
    setLogoutButton: (button) => {
      logoutButton = button
    },
  }
}

test('only the trusted native parent enables embedded navigation', () => {
  const h = createHarness()
  assert.equal(h.messages.at(-1)[0].type, 'native-navigation-context-request')
  assert.equal(h.messages.at(-1)[1], 'http://tauri.localhost')
  h.sendContext({ source: {} })
  h.sendContext({ origin: 'https://evil.example' })
  h.sendContext({ data: { type: 'native-navigation-context', source: 'other', unified: true } })
  assert.equal(h.document.documentElement.dataset.sunshineNavigation, undefined)
  h.sendContext()
  assert.equal(h.document.documentElement.dataset.sunshineNavigation, 'unified')
  const standalone = createHarness({ embedded: false })
  assert.equal(
    standalone.messages.some(([message]) => message.type === 'native-navigation-context-request'),
    false,
  )
  standalone.sendContext()
  assert.equal(standalone.document.documentElement.dataset.sunshineNavigation, undefined)
})

test('the Panel no longer drives the WebUI log out: #logout opens nothing and leaves the address alone', () => {
  const h = createHarness({ pathname: '/password', hash: '#logout' })
  h.setLogoutButton({ click: () => assert.fail('the Panel must not click the WebUI log out') })
  h.sendContext()
  h.mutate()
  assert.equal(h.historyCalls.length, 0)
  assert.doesNotMatch(script, /logout/i, 'the injected script has no log out branch left')
})

test('embedded config tabs scroll horizontally without moving the page and restore the original behavior on revocation', () => {
  const h = createHarness()
  const scrolls = []
  const list = {
    scrollWidth: 1000,
    clientWidth: 300,
    scrollLeft: 20,
    getBoundingClientRect: () => ({ left: 0 }),
    scrollTo: (options) => scrolls.push(options),
  }
  const original = () => assert.fail('vertical scrolling must not run in embedded mode')
  const tab = {
    scrollIntoView: original,
    closest: () => list,
    getBoundingClientRect: () => ({ left: 270, width: 80 }),
  }
  h.tabs.push(tab)
  h.sendContext()
  tab.scrollIntoView({ behavior: 'smooth', block: 'nearest' })
  assert.equal(scrolls[0].left, 180)
  assert.equal(scrolls[0].behavior, 'smooth')
  list.scrollWidth = 300
  tab.scrollIntoView()
  assert.equal(scrolls.length, 1)
  h.mutate()
  h.sendContext({ data: { type: 'native-navigation-context', source: 'sunshine-control-panel', unified: false } })
  assert.equal(tab.scrollIntoView, original)
  assert.equal(h.document.documentElement.dataset.sunshineNavigation, undefined)
})
