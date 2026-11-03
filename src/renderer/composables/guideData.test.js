import './_testing/foundationUi.mjs'
import test from 'node:test'
import assert from 'node:assert/strict'

const { createGuideData, shouldOpenGuide } = await import('./guideData.js')
const { buildGuideSteps, readProgress, writeProgress } = await import('foundation-ui/vue/guide.js')

// A host behind the panel's proxy: routes map a path to a body, a function or a Response
function host(routes, calls = []) {
  return async (url, init = {}) => {
    const path = String(url).replace('http://proxy', '')
    calls.push({ path, method: init.method || 'GET', body: typeof init.body === 'string' ? JSON.parse(init.body) : undefined })
    const entry = routes[path]
    if (entry === undefined) return new Response('nf', { status: 404 })
    const value = typeof entry === 'function' ? await entry(path, init) : entry
    return value instanceof Response ? value : new Response(JSON.stringify(value), { status: 200, headers: { 'content-type': 'application/json' } })
  }
}
const make = (routes, extra = {}, calls = []) => createGuideData({ proxyUrl: async () => 'http://proxy', fetchImpl: host(routes, calls), vddStatus: async () => ({ installed: true, running: true, state: 'ready' }), openUrl() {}, copy() {}, openApps() {}, navigate() {}, redirectMs: 0, ...extra })
const memory = () => { const m = new Map(); return { getItem: (k) => (m.has(k) ? m.get(k) : null), setItem: (k, v) => m.set(k, String(v)), removeItem: (k) => m.delete(k) } }

test('a host with an account answers the config, one without sends a page: the account step follows', async () => {
  const withAccount = make({ '/api/config': { version: '1', platform: 'windows' } })
  await withAccount.probeAccount()
  assert.equal(withAccount.state.account.exists, true)
  assert.equal(buildGuideSteps({ skip: withAccount.state.account.exists ? ['account'] : [] })[0].id, 'basics')

  const without = make({ '/api/config': () => new Response('<html>create account</html>', { status: 200 }) })
  await without.probeAccount()
  assert.equal(without.state.account.exists, false)
  assert.equal(buildGuideSteps({ skip: without.state.account.exists ? ['account'] : [] })[0].id, 'account')

  const down = make({})
  await down.probeAccount()
  assert.equal(down.state.account.exists, false)
})

test('creating the account posts the credentials and the guide goes on: this computer needs no sign-in', async () => {
  const calls = []
  const data = make({ '/api/password': { status: 'true' } }, {}, calls)
  const result = await data.createAccount((key) => key)({ username: 'sunshine', password: 'pw', confirm: 'pw' })
  assert.deepEqual(result, { ok: true })
  assert.deepEqual(calls[0].body, { newUsername: 'sunshine', newPassword: 'pw', confirmNewPassword: 'pw' })
})

test('the account step reports the host error or a network failure', async () => {
  const refused = make({ '/api/password': { status: false, error: 'Too short' } })
  assert.deepEqual(await refused.createAccount((key) => `T:${key}`)({ username: 'a', password: 'b', confirm: 'b' }), { ok: false, error: 'T:error Too short' })
  const empty = make({ '/api/password': { status: false } })
  assert.deepEqual(await empty.createAccount((key) => `T:${key}`)({}), { ok: false, error: 'T:error T:server_error' })
  const offline = createGuideData({ proxyUrl: async () => 'http://proxy', fetchImpl: async () => { throw new Error('offline') }, navigate() {} })
  assert.deepEqual(await offline.createAccount((key) => `T:${key}`)({}), { ok: false, error: 'T:error offline' })
})

test('the setup step reads the host and writes only the chosen items back', async () => {
  const calls = []
  const data = make({ '/api/config': (path, init) => (init.method === 'POST' ? { status: true } : { platform: 'windows', locale: 'en', port: 47989, setup_wizard_completed: 'false', adapters: [{ name: 'RTX' }], display_devices: [{ device_id: 'D1', data: 'FRIENDLY NAME: F32' }], pair_name: 'My phone' }) }, {}, calls)
  await data.callbacks.loadSetup()
  assert.equal(data.state.setup.completed, false)
  assert.equal(data.state.setup.adapters.length, 1)
  assert.equal(data.state.pair.name, 'My phone')
  assert.deepEqual(await data.callbacks.saveSetup({ adapter: 'RTX', display: 'D1' }), { ok: true })
  const posted = calls.filter((c) => c.method === 'POST').at(-1).body
  assert.deepEqual(posted, { platform: 'windows', locale: 'en', port: 47989, setup_wizard_completed: true, adapters: [{ name: 'RTX' }], display_devices: [{ device_id: 'D1', data: 'FRIENDLY NAME: F32' }], pair_name: 'My phone', adapter_name: 'RTX', output_name: 'D1', display_device_prep: 'ensure_only_display' })
  assert.equal(data.state.setup.completed, true)
})

test('the virtual display needs a ready driver, a physical one does not', async () => {
  const routes = { '/api/config': (path, init) => (init.method === 'POST' ? { status: true } : { platform: 'windows' }) }
  const notReady = make(routes, { vddStatus: async () => ({ installed: true, running: false, state: 'stopped' }) })
  await notReady.callbacks.loadSetup()
  assert.deepEqual(await notReady.callbacks.saveSetup({ adapter: '', display: 'ZakoHDR' }), { ok: false, vdd: false })
  assert.deepEqual(await notReady.callbacks.saveSetup({ adapter: '', display: 'D1' }), { ok: true })
  const broken = make(routes, { vddStatus: async () => { throw new Error('no command') } })
  await broken.callbacks.loadSetup()
  assert.deepEqual(await broken.callbacks.saveSetup({ adapter: '', display: 'ZakoHDR' }), { ok: false, vdd: false })
})

test('skipping the setup only sets the done flag', async () => {
  const calls = []
  const data = make({ '/api/config': (path, init) => (init.method === 'POST' ? { status: true } : { locale: 'de', output_name: 'X', setup_wizard_completed: false }) }, {}, calls)
  await data.callbacks.skipSetup()
  assert.deepEqual(calls.at(-1).body, { locale: 'de', output_name: 'X', setup_wizard_completed: true })
  assert.equal(data.state.setup.completed, true)
})

test('addresses, clients and sessions come from the host, bad answers leave them empty', async () => {
  const good = make({
    '/api/network/listeners': { status: true, addresses: [{ name: 'Ethernet', address: '192.168.1.5' }] },
    '/api/clients/list': { named_certs: [{ name: 'Pixel 9', uuid: 'u1' }] },
    '/api/runtime/sessions': { sessions: [{ client_name: 'Pixel 9', app_name: 'Desktop', width: 1920, height: 1080, fps: 60 }] },
  })
  await Promise.all([good.callbacks.refreshAddresses(), good.callbacks.refreshClients(), good.callbacks.refreshSessions()])
  assert.equal(good.state.host.addresses[0].address, '192.168.1.5')
  assert.equal(good.state.clients[0].name, 'Pixel 9')
  assert.equal(good.state.sessions[0].client_name, 'Pixel 9')

  const bad = make({ '/api/network/listeners': { status: false }, '/api/clients/list': { named_certs: 'x' }, '/api/runtime/sessions': () => new Response('{}', { status: 403 }) }, { activeSessions: async () => { throw new Error('no command') } })
  await Promise.all([bad.callbacks.refreshAddresses(), bad.callbacks.refreshClients(), bad.callbacks.refreshSessions()])
  assert.equal(bad.state.host.addresses, null)
  assert.equal(bad.state.clients, null)
  assert.equal(bad.state.sessions, null)
})

test('sessions fall back to the panel command when the route is not there', async () => {
  const data = make({ '/api/runtime/sessions': () => new Response('{}', { status: 404 }) }, { activeSessions: async () => [{ client_name: 'iPad', app_name: 'Desktop', width: 1920, height: 1080, fps: 60 }] })
  await data.callbacks.refreshSessions()
  assert.equal(data.state.sessions[0].client_name, 'iPad')
})

test('pairing sends the PIN and the name; the new client shows in the list', async () => {
  const clients = [{ name: 'Old', uuid: 'a' }]
  const calls = []
  const data = make({
    '/api/pin': (path, init) => { const body = JSON.parse(init.body); if (body.pin === '0000') return { status: false, error: 'Wrong PIN' }; clients.push({ name: body.name, uuid: 'b' }); return { status: true } },
    '/api/clients/list': () => ({ named_certs: clients }),
  }, {}, calls)
  await data.callbacks.refreshClients()
  const before = data.state.clients.map((c) => c.uuid)
  assert.deepEqual(await data.callbacks.pair('0000', 'Pixel'), { ok: false, error: 'Wrong PIN' })
  assert.deepEqual(await data.callbacks.pair('1234', 'Pixel'), { ok: true })
  await data.callbacks.refreshClients()
  assert.deepEqual(data.state.clients.filter((c) => !before.includes(c.uuid)).map((c) => c.name), ['Pixel'])
  assert.deepEqual(calls.find((c) => c.path === '/api/pin' && c.body.pin === '1234').body, { pin: '1234', name: 'Pixel' })
  const offline = createGuideData({ proxyUrl: async () => 'http://proxy', fetchImpl: async () => { throw new Error('offline') } })
  assert.deepEqual(await offline.callbacks.pair('1234', 'x'), { ok: false, error: 'offline' })
})

test('opening the apps page goes through the injected navigation with the request in the address', () => {
  const asked = []
  const data = make({}, { openApps: (mode) => asked.push(mode) })
  data.callbacks.openApps('scan')
  data.callbacks.openApps('restore')
  assert.deepEqual(asked, ['scan', 'restore'])
})

test('first run: setup not completed and nobody paired opens the guide', async () => {
  const routes = (setup, named) => host({ '/api/config': { setup_wizard_completed: setup }, '/api/clients/list': { named_certs: named } })
  const ask = (fetchImpl, extra = {}) => shouldOpenGuide({ proxyUrl: async () => 'http://proxy', fetchImpl, progress: { finished: false }, ...extra })
  assert.equal(await ask(routes(false, [])), true)
  assert.equal(await ask(routes('false', [])), true)
  assert.equal(await ask(routes(true, [])), false)
  assert.equal(await ask(routes(false, [{ name: 'Pixel', uuid: '1' }])), false)
  assert.equal(await ask(routes(false, []), { progress: { finished: true } }), false)
  assert.equal(await ask(routes(false, []), { dismissed: true }), false)
})

test('a host that has no account yet sends the page for creating it: the first run opens, at the account step', async () => {
  const page = host({ '/api/config': () => new Response('<html>create account</html>', { status: 200, headers: { 'content-type': 'text/html; charset=utf-8' } }) })
  const ask = (extra = {}) => shouldOpenGuide({ proxyUrl: async () => 'http://proxy', fetchImpl: page, progress: { finished: false }, ...extra })
  assert.equal(await ask(), true)
  assert.equal(await ask({ dismissed: true }), false)
  assert.equal(await ask({ progress: { finished: true } }), false)
})

test('a host that answers badly or not at all never opens the guide', async () => {
  const ask = (fetchImpl) => shouldOpenGuide({ proxyUrl: async () => 'http://proxy', fetchImpl, progress: { finished: false } })
  assert.equal(await ask(host({})), false)
  assert.equal(await ask(host({ '/api/config': { setup_wizard_completed: false } })), false)
  assert.equal(await ask(host({ '/api/config': { setup_wizard_completed: false }, '/api/clients/list': { named_certs: 'x' } })), false)
  assert.equal(await ask(host({ '/api/config': { setup_wizard_completed: false }, '/api/clients/list': { status: false, named_certs: [] } })), false)
  assert.equal(await ask(host({ '/api/config': () => new Response('<html>', { status: 200 }) })), false)
  assert.equal(await ask(async () => { throw new Error('refused') }), false)
  assert.equal(await ask(host({ '/api/config': () => new Response('<html>create account</html>', { status: 500, headers: { 'content-type': 'text/html' } }) })), false)
  assert.equal(await shouldOpenGuide({ proxyUrl: async () => { throw new Error('no proxy') }, fetchImpl: host({}) }), false)
})

test('the panel keeps its progress under its own key', () => {
  const storage = memory()
  writeProgress(storage, 'foundation.guide.panel', { step: 'pair', visited: ['setup', 'pair'] })
  assert.equal(readProgress(storage, 'foundation.guide.panel').step, 'pair')
  assert.equal(readProgress(storage, 'foundation.guide.web').step, null)
})
