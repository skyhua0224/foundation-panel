import assert from 'node:assert/strict'
import test from 'node:test'
import { framedMove } from './frameFollow.js'

// the answers of the panel's real tables (useRouter.js routeForWebPath / routeOfPlace) for the pages that matter here
const PATHS = { '/pin': 'web-clients', '/clients': 'web-clients', '/apps': 'web-apps', '/downloads': 'web-downloads', '/password': 'web-password', '/troubleshooting': 'diagnostics', '/diagnostics': 'diagnostics', '/dualsense': 'gamepad', '/': 'home', '/config': 'web-config' }
const PLACES = { 'display:output': 'web-display-output', 'display:modes': 'web-display-modes', 'display:encoders': 'web-display-encoders', 'audio:sound': 'web-audio-sound', 'input:gamepad': 'gamepad' }
const parseHash = (hash) => { const q = new URLSearchParams(String(hash).replace(/^#/, '')); return q.get('tab') ? { tab: q.get('tab'), ...(q.get('sub') ? { sub: q.get('sub') } : {}), ...(q.get('focus') ? { focus: q.get('focus') } : {}) } : (hash === '#bind_address' ? { tab: 'network', focus: 'bind_address' } : null) }
const deps = {
  configRoute: 'web-config',
  parseHash,
  routeOfPlace: (tab, sub) => PLACES[`${tab}:${sub}`] || null,
  routeForWebPath: (path) => PATHS[String(path).split(/[?#]/, 1)[0].replace(/\.html$/, '').replace(/\/$/, '') || '/'] || 'home',
}
const move = (path, route, params) => framedMove(path, { route, params }, deps)

test('a link in 网络 settings to the client list: the panel follows to 客户端', () => {
  assert.deepEqual(move('/pin#client-list', 'web-config', { tab: 'network' }), { route: 'web-clients', params: undefined })
})
test('a link from the clients page to the network question: the settings page at the network tab', () => {
  assert.deepEqual(move('/config#bind_address', 'web-clients'), { route: 'web-config', params: { tab: 'network', focus: 'bind_address' } })
})
test('the address the panel itself asked for is no move', () => {
  assert.equal(move('/pin', 'web-clients'), null)
  assert.equal(move('/pin#pairing', 'web-clients'), null)
  assert.equal(move('/config#tab=display&sub=output', 'web-display-output', { tab: 'display', sub: 'output' }), null)
  assert.equal(move('/config#tab=network', 'web-config', { tab: 'network' }), null)
})
test('a place set by the hash inside the open settings page moves the list: display → audio, and to a tab of the settings page', () => {
  assert.deepEqual(move('/config#tab=audio&sub=sound', 'web-display-output', { tab: 'display', sub: 'output' }), { route: 'web-audio-sound', params: { tab: 'audio', sub: 'sound' } })
  assert.deepEqual(move('/config#tab=display&sub=encoders', 'web-audio-sound'), { route: 'web-display-encoders', params: { tab: 'display', sub: 'encoders' } })
  assert.deepEqual(move('/config#tab=files', 'web-display-output'), { route: 'web-config', params: { tab: 'files' } })
  assert.deepEqual(move('/config#tab=files', 'web-config', { tab: 'network' }), { route: 'web-config', params: { tab: 'files' } })
})
test('a bare /config names no tab: nothing to follow', () => {
  assert.equal(move('/config', 'web-display-output'), null)
  assert.equal(move('/config', 'web-clients'), null)
})
test('a page of the panel that stands for a Web path (diagnostics, the password page) is followed too', () => {
  assert.deepEqual(move('/troubleshooting', 'web-config', { tab: 'files' }), { route: 'diagnostics', params: undefined })
  assert.deepEqual(move('/password', 'web-config', { tab: 'network' }), { route: 'web-password', params: undefined })
})
test('addresses that are not pages of the panel are no move: login, welcome, a file, a foreign address', () => {
  for (const p of ['/login', '/welcome', '/assets/web/x.js', '//evil.example/pin', 'http://x/pin', '', undefined]) assert.equal(move(p, 'web-clients'), null, String(p))
})
test('the index of the Web is the panel overview', () => {
  assert.deepEqual(move('/', 'web-clients'), { route: 'home', params: undefined })
  assert.deepEqual(move('/index.html', 'web-apps'), { route: 'home', params: undefined })
})
test('.html and a trailing slash name the same pages', () => {
  assert.deepEqual(move('/apps.html', 'web-clients'), { route: 'web-apps', params: undefined })
  assert.equal(move('/pin/', 'web-clients'), null)
})
