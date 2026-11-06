import assert from 'node:assert/strict'
import test from 'node:test'
import { BLANK, framePageGo, routeChanged, webuiNavigated } from './frameRoute.js'

// the route tables of the panel, as useRouter has them for the destinations that matter here
const WEB = { 'web-clients': '/pin', 'web-apps': '/apps', 'web-downloads': '/downloads', 'web-config': '/config', 'web-troubleshooting': '/troubleshooting' }
const PATHS = Object.fromEntries(Object.entries(WEB).map(([route, path]) => [path, route]))
const deps = { urlOf: (path) => (path ? `http://127.0.0.1:48081${path}` : ''), webPathOf: (route) => WEB[route] || null, routeForWebPath: (path) => PATHS[String(path).split(/[?#]/)[0]] || 'home' }
const frame = (over = {}) => ({ currentPath: '/pin', url: 'http://127.0.0.1:48081/pin', saved: '', ...over })

test('the rail of 串流: the frame was asked for /pin, then the route changed: the frame stays on /pin', () => {
  const asked = webuiNavigated('/pin', frame({ currentPath: '/', url: BLANK }), deps)
  assert.equal(asked.url, 'http://127.0.0.1:48081/pin')
  assert.equal(routeChanged({ from: 'home', to: 'web-clients' }, { ...asked, url: asked.url }, deps), null)
})
test('leaving the overview for a Web page never leaves the frame blank', () => {
  for (const [route, path] of Object.entries(WEB)) {
    const d = routeChanged({ from: 'home', to: route }, frame({ currentPath: path }), deps)
    assert.ok(d === null || d.url !== BLANK, `${route} is not put to sleep`)
  }
})
test('a frame that sleeps is woken for a Web page, on the page the route stands for', () => {
  const asleep = frame({ url: BLANK, saved: 'http://127.0.0.1:48081/apps', currentPath: '/apps' })
  const d = routeChanged({ from: 'about', to: 'web-clients' }, asleep, deps)
  assert.deepEqual(d, { kind: 'show', url: 'http://127.0.0.1:48081/pin', currentPath: '/pin', saved: '' })
  // the same page it was on, with its hash, is kept
  const same = routeChanged({ from: 'about', to: 'web-clients' }, frame({ url: BLANK, currentPath: '/pin#pairing' }), deps)
  assert.equal(same.url, 'http://127.0.0.1:48081/pin#pairing')
})
test('a panel page after the overview puts the frame to sleep and the overview wakes it', () => {
  const asleep = routeChanged({ from: 'home', to: 'panel-settings' }, frame(), deps)
  assert.equal(asleep.kind, 'suspend'); assert.equal(asleep.url, BLANK); assert.equal(asleep.saved, 'http://127.0.0.1:48081/pin')
  const back = routeChanged({ from: 'panel-settings', to: 'home' }, frame({ url: BLANK, saved: asleep.saved }), deps)
  assert.deepEqual(back, { kind: 'resume', url: 'http://127.0.0.1:48081/pin', saved: '' })
})
test('with no address for the frame yet nothing is decided', () => {
  assert.equal(routeChanged({ from: 'home', to: 'web-clients' }, frame({ url: BLANK }), { ...deps, urlOf: () => '' }), null)
})
test('every transition between destinations ends with a frame that is not blank when the destination is a Web page', () => {
  const routes = ['home', 'panel-settings', 'help', 'about', 'diagnostics', ...Object.keys(WEB)]
  for (const from of routes) for (const to of routes) {
    if (from === to) continue
    for (const start of [frame(), frame({ url: BLANK, saved: 'http://127.0.0.1:48081/apps', currentPath: '/apps' }), frame({ url: BLANK, currentPath: '/pin' })]) {
      // the order of the rail: the frame is asked first (when the destination is a Web page), then the route changes
      let state = start
      if (WEB[to]) { const asked = webuiNavigated(WEB[to], state, deps); state = { ...state, currentPath: asked.currentPath, url: asked.url ?? state.url, saved: asked.saved } }
      const d = routeChanged({ from, to }, state, deps)
      if (d) state = { ...state, url: d.url, saved: d.saved ?? state.saved, currentPath: d.currentPath ?? state.currentPath }
      if (WEB[to]) assert.notEqual(state.url, BLANK, `${from} -> ${to} from ${start.url}`)
    }
  }
})
test('asking for the page the frame already shows does nothing; asking while it sleeps loads it', () => {
  assert.equal(webuiNavigated('/pin', frame(), deps).url, null)
  assert.equal(webuiNavigated('/pin', frame({ url: BLANK }), deps).url, 'http://127.0.0.1:48081/pin')
  assert.equal(webuiNavigated('/apps', frame(), deps).url, 'http://127.0.0.1:48081/apps')
})

test('another framed page is a message to the open frame when its Web can be told (one document, no new address); otherwise the address changes as before', () => {
  const open = { goReady: true, url: 'http://127.0.0.1:48081/pin', currentPath: '/pin' }
  for (const to of ['/apps', '/downloads', '/config', '/config#tab=network', '/password']) assert.equal(framePageGo(to, open), true, to)
  assert.equal(framePageGo('/apps', { ...open, goReady: false }), false, 'an older Web in the frame has not said that it can be told')
  assert.equal(framePageGo('/apps', { ...open, url: BLANK }), false, 'a sleeping frame is loaded, not told')
  assert.equal(framePageGo('/apps', { ...open, url: 'tauri://localhost/x' }), false)
  assert.equal(framePageGo('/pin', open), false, 'the same page again is not a message')
  assert.equal(framePageGo('/config#tab=av', { ...open, currentPath: '/config#tab=general' }), false, 'a place inside /config has its own message')
  assert.equal(framePageGo('/troubleshooting', open), false, 'a page that is not one of the framed ones')
  assert.equal(framePageGo('/apps', { ...open, currentPath: '/' }), false, 'the overview is not a framed page')
})
