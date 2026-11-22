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
  assert.equal(buildGuideSteps({ skip: withAccount.state.account.exists ? ['account'] : [] })[0].id, 'intro') // the first screen (the project and the language) comes first, with or without the account step
  assert.ok(!buildGuideSteps({ skip: ['account'] }).some((s) => s.id === 'account'))

  const without = make({ '/api/config': () => new Response('<html>create account</html>', { status: 200 }) })
  await without.probeAccount()
  assert.equal(without.state.account.exists, false)
  assert.deepEqual(buildGuideSteps({ skip: without.state.account.exists ? ['account'] : [] }).slice(0, 3).map((s) => s.id), ['intro', 'account', 'channel'])

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
    '/api/network/listeners': { status: true, addresses: [{ name: 'Ethernet', address: '192.0.2.5' }] },
    '/api/clients/list': { named_certs: [{ name: 'Pixel 9', uuid: 'u1' }] },
    '/api/runtime/sessions': { sessions: [{ client_name: 'Pixel 9', app_name: 'Desktop', width: 1920, height: 1080, fps: 60 }] },
  })
  await Promise.all([good.callbacks.refreshAddresses(), good.callbacks.refreshClients(), good.callbacks.refreshSessions()])
  assert.equal(good.state.host.addresses[0].address, '192.0.2.5')
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

test('the Web UI access step reads the saved scope and bind address and saves one key without touching the rest', async () => {
  const calls = []
  const data = make({
    '/api/config': { origin_web_ui_allowed: 'wan', bind_address: '192.0.2.5', port: '57989', locale: 'zh' },
    '/api/network/listeners': { status: true, active: { origin_web_ui_allowed: 'lan', bind_address: '' }, saved: { origin_web_ui_allowed: 'wan', bind_address: '192.0.2.5' }, addresses: [] },
  }, {}, calls)
  await data.callbacks.loadWebAccess()
  assert.equal(data.state.webAccess.scope, 'wan')
  assert.equal(data.state.webAccess.bind, '192.0.2.5')
  assert.equal(data.state.webAccess.active.origin_web_ui_allowed, 'lan')
  assert.equal(data.state.webAccess.saved.origin_web_ui_allowed, 'wan')
  const answer = await data.callbacks.saveWebAccess({ origin_web_ui_allowed: 'pc' })
  assert.deepEqual(answer, { ok: true })
  const post = calls.find((c) => c.method === 'POST')
  assert.equal(post.path, '/api/config')
  assert.deepEqual(post.body, { origin_web_ui_allowed: 'pc', bind_address: '192.0.2.5', port: '57989', locale: 'zh' })
  assert.equal(data.state.webAccess.scope, 'pc')
  assert.ok(buildGuideSteps().at(-1).optional.includes('webaccess'), 'the page is an optional one of the done page')
})

test('an upgrade: a host that is set up opens the guide once per new version of the panel', async () => {
  const routes = (named) => host({ '/api/config': { setup_wizard_completed: true }, '/api/clients/list': { named_certs: named } })
  const ask = (extra = {}) => shouldOpenGuide({ proxyUrl: async () => 'http://proxy', fetchImpl: routes([{ name: 'Pixel', uuid: '1' }]), progress: null, version: '2.4.0', ...extra })
  assert.equal(await ask(), true)
  assert.equal(await ask({ progress: { finished: true, visited: [], version: '2.3.0', mode: 'fresh' } }), true)
  assert.equal(await ask({ progress: { finished: false, visited: ['whatsnew'], version: '2.4.0', mode: 'upgrade' } }), false, 'closed or skipped for this version')
  assert.equal(await ask({ progress: { finished: true, visited: [], version: '2.4.0', mode: 'upgrade' } }), false)
  assert.equal(await ask({ dismissed: true }), false)
  assert.equal(await ask({ version: '' }), false, 'without a version of the panel only a first run opens it')
})

test('the guide scans the game libraries on the host and adds the chosen games to the apps list', async () => {
  const calls = []
  const fetchImpl = async (url, init = {}) => {
    const path = String(url).replace('http://proxy', '')
    calls.push({ path, method: init.method || 'GET', body: init.body ? JSON.parse(init.body) : null })
    if (path === '/api/apps/scan') return new Response(JSON.stringify({ games: [{ name: 'Portal', platform: 'steam', launch: 'steam://run/400', install_dir: 'C:/Steam/Portal' }] }), { status: 200 })
    if (path === '/api/apps') return new Response(JSON.stringify(init.method === 'POST' ? { status: true } : { apps: [{ name: 'Desktop' }] }), { status: 200 })
    return new Response('{}', { status: 404 })
  }
  const data = createGuideData({ proxyUrl: async () => 'http://proxy', fetchImpl })
  const scan = await data.callbacks.scanGames(['steam'])
  assert.equal(scan.ok, true)
  assert.deepEqual(calls[0].body, { sources: ['steam'] })
  assert.equal(scan.games[0].name, 'Portal')
  assert.deepEqual(await data.callbacks.addGames(scan.games), { ok: true, added: 1 })
  const saved = calls.filter((c) => c.method === 'POST' && c.path === '/api/apps').at(-1).body.apps
  assert.deepEqual(saved.map((a) => a.name), ['Desktop', 'Portal'])
  assert.equal(saved[1].cmd, 'steam://run/400')
  const old = createGuideData({ proxyUrl: async () => 'http://proxy', fetchImpl: async () => new Response('', { status: 404 }) })
  assert.equal((await old.callbacks.scanGames(['steam'])).unsupported, true)
})

test('the test tools are opened through the injected navigation', () => {
  const asked = []
  make({}, { openTools: () => asked.push('tools') }).callbacks.openTools()
  assert.deepEqual(asked, ['tools'])
})

test('probeHost reads the start time of the host process; an older host without /api/version still counts as up; a host that is away does not', async () => {
  const a = make({ '/api/version': { process_started_at: '2026-11-07T05:00:00Z' } })
  assert.deepEqual(await a.callbacks.probeHost(), { ok: true, startedAt: '2026-11-07T05:00:00Z' })
  const old = make({ '/api/network/listeners': { status: true } })
  assert.deepEqual(await old.callbacks.probeHost(), { ok: true, startedAt: '' })
  const away = make({ '/api/version': () => new Response('', { status: 503 }) })
  assert.deepEqual(await away.callbacks.probeHost(), { ok: false })
  const dead = createGuideData({ proxyUrl: async () => 'http://proxy', fetchImpl: async () => { throw new Error('refused') }, navigate() {} })
  assert.deepEqual(await dead.callbacks.probeHost(), { ok: false })
})

test('the check of the guide passes an unsaved address on to the shared check', async () => {
  const seen = []
  const data = make({}, { reach: { view: { report: null, status: 'idle' }, refresh: async (force, options) => { seen.push([force, options]) } } })
  await data.callbacks.checkReach(true, { test: 'home.example.net' })
  assert.deepEqual(seen, [[true, { test: 'home.example.net' }]])
})

test('「还原内置应用」 in the flow restores through the host API and answers in place, without leaving the guide', async () => {
  const calls = []
  const saved = []
  const routes = {
    '/api/apps': (path, init) => { if (init.method === 'POST') { saved.push(JSON.parse(init.body)); return { status: 'true' } } return { apps: [{ name: 'Desktop', cmd: 'broken.exe' }, { name: 'Mine', cmd: 'mine.exe' }] } },
    '/api/config': { platform: 'windows' },
  }
  const asked = []
  const data = make(routes, { openApps: (mode) => asked.push(mode) }, calls)
  assert.equal(typeof data.callbacks.restoreApps, 'function', 'with it the flow never leaves for the apps page')
  assert.deepEqual(await data.callbacks.restoreApps(), { ok: true, result: 'done' })
  assert.deepEqual(asked, [], 'the Web UI page is not opened')
  assert.equal(saved.length, 1)
  assert.equal(saved[0].editApp, null)
  assert.deepEqual(saved[0].apps.map((a) => a.name), ['Desktop', 'Mine', 'Steam Big Picture', 'Xbox Game'], 'the person\'s own app stays; the missing built-ins are added')
  assert.equal(saved[0].apps[0].cmd, undefined, 'Desktop is back to its default')
  // already as the defaults: nothing is written, the flow says so
  const { getDefaultBuiltInApps } = await import('foundation-ui/apps')
  const same = make({ ...routes, '/api/apps': (path, init) => { if (init.method === 'POST') throw new Error('must not write'); return { apps: getDefaultBuiltInApps('windows') } } })
  assert.deepEqual(await same.callbacks.restoreApps(), { ok: true, result: 'none' })
  // the host refuses or is gone: the flow shows the failure
  const refuse = make({ ...routes, '/api/apps': (path, init) => (init.method === 'POST' ? new Response('no', { status: 500 }) : { apps: [] }) })
  assert.deepEqual(await refuse.callbacks.restoreApps(), { ok: false })
  const down = createGuideData({ proxyUrl: async () => 'http://proxy', fetchImpl: async () => { throw new Error('offline') } })
  assert.deepEqual(await down.callbacks.restoreApps(), { ok: false, error: 'offline' })
})

test('the address step reads the port the host runs and the one it saved, and the name clients list', async () => {
  const data = make({ '/api/network/listeners': { status: true, active: { port: 47989, upnp: false }, saved: { port: 48989, upnp: false }, addresses: [{ name: '以太网', address: '192.0.2.5' }] }, '/api/config': { sunshine_name: 'SKY-PC', external_endpoints: '' } })
  await data.callbacks.loadNetwork()
  assert.deepEqual([data.state.network.port, data.state.network.activePort, data.state.host.name], [48989, 47989, 'SKY-PC'])
  const old = make({ '/api/network/listeners': { status: true, active: { port: 47989 }, saved: {}, addresses: [] }, '/api/config': {} })
  await old.callbacks.loadNetwork()
  assert.deepEqual([old.state.network.port, old.state.network.activePort, old.state.host.name], [47989, 47989, ''])
})

test('setPort saves the base port in the configuration through the host route, nothing else changes', async () => {
  const calls = []
  const data = make({ '/api/config': (path, init) => (init.method === 'POST' ? { status: true } : { sunshine_name: 'SKY-PC', port: 47989, upnp: 'enabled' }) }, {}, calls)
  assert.deepEqual(await data.callbacks.setPort(48989), { ok: true })
  const post = calls.find((c) => c.method === 'POST')
  assert.equal(post.path, '/api/config'); assert.deepEqual(post.body, { sunshine_name: 'SKY-PC', port: 48989, upnp: 'enabled' })
  assert.equal(data.state.network.port, 48989)
  const refused = make({ '/api/config': (path, init) => (init.method === 'POST' ? new Response(JSON.stringify({ status: false, error: 'port out of range' }), { status: 400 }) : {}) })
  assert.deepEqual(await refused.callbacks.setPort(80), { ok: false, error: 'port out of range' })
})

test('the address step opens the diagnostics through the panel', async () => {
  let opened = 0
  const data = make({}, { openDiagnostics: () => { opened += 1 } })
  data.callbacks.openDiagnostics(); assert.equal(opened, 1)
  assert.doesNotThrow(() => make({}).callbacks.openDiagnostics())
})

test('the guide reads what the host remembers and saves the steps done and who runs it, one write at a time', async () => {
  const config = { setup_wizard_completed: false, guide_done: 'intro,account' }
  const posts = []
  const fetchImpl = async (url, init) => {
    if (init?.method === 'POST') { posts.push(JSON.parse(init.body)); Object.assign(config, JSON.parse(init.body)); return { ok: true, json: async () => ({ status: true }) } }
    return { ok: true, headers: { get: () => 'application/json' }, json: async () => ({ ...config }) }
  }
  const data = createGuideData({ proxyUrl: async () => 'http://h', fetchImpl, openUrl() {}, copy() {}, openApps() {} })
  assert.deepEqual((await data.loadGuideHost()).done, ['intro', 'account'])
  await data.callbacks.saveGuideProgress({ done: ['intro', 'account', 'appearance'], active: { device: 'phone', at: 100 } })
  assert.equal(posts.length, 1)
  assert.equal(posts[0].guide_done, 'intro,account,appearance')
  assert.equal(posts[0].guide_active, 'phone@100')
  assert.equal(posts[0].setup_wizard_completed, false, 'the rest of the configuration is kept')
})

test('the panel does not open the guide while a phone runs it, nor after it was closed on any device', async () => {
  const now = Date.now()
  const host = (extra) => async (url) => ({ ok: true, headers: { get: () => 'application/json' }, json: async () => (/clients/.test(url) ? { status: true, named_certs: [] } : { setup_wizard_completed: false, ...extra }) })
  const ask = (extra, device) => shouldOpenGuide({ proxyUrl: async () => 'http://h', fetchImpl: host(extra), progress: null, version: '1.3.9' })
  assert.equal(await ask({}), true)
  assert.equal(await ask({ guide_active: `phone@${Math.floor(now / 1000) - 10}` }), false)
  assert.equal(await ask({ guide_closed: '1.3.9' }), false)
  assert.equal(await ask({ guide_finished: 'true', guide_closed: '1.3.9' }), false)
})

test('the port check of the address step is the host 体检 (network group), in the language of the guide, through the proxy', async () => {
  const calls = []
  const report = { version: 1, checked_at: 'x', groups: [{ id: 'network', title: 'Network', items: ['network.lan', 'network.ports'] }], items: [{ id: 'network.lan', group: 'network', status: 'pass', title: 'Local network', detail: 'listening' }, { id: 'network.ports', group: 'network', status: 'fail', title: 'Ports', detail: 'UDP 47998' }] }
  const data = make({ '/api/doctor?only=network&lang=en': report }, { locale: () => 'en-US' }, calls)
  const r = await data.callbacks.checkPorts()
  assert.equal(r.ok, true); assert.equal(r.worst, 'fail'); assert.deepEqual(r.items.map((i) => i.id), ['network.lan', 'network.ports'])
  assert.deepEqual(calls.map((c) => c.path), ['/api/doctor?only=network&lang=en'])
  const zh = await make({ '/api/doctor?only=network&lang=zh': report }, { locale: () => 'zh' }).callbacks.checkPorts()
  assert.equal(zh.ok, true, 'a Chinese guide asks for Chinese words')
  assert.deepEqual(await make({}, {}).callbacks.checkPorts(), { ok: false, unsupported: true }, 'a host without the route (404) is unsupported, not failed')
  const down = createGuideData({ proxyUrl: async () => 'http://proxy', fetchImpl: async () => { throw new Error('away') }, vddStatus: async () => ({}), openUrl() {}, copy() {} })
  assert.deepEqual(await down.callbacks.checkPorts(), { ok: false, unsupported: false })
})

test('the update channel step reads the panel preference and saves a choice at once, answering { ok } so the step can say it was not saved', async () => {
  const saved = []
  const data = make({}, { channel: { get: async () => true, set: async (include) => { saved.push(include) } } })
  assert.equal(data.state.channel.value, 'stable', 'the default until the preference is read')
  await data.callbacks.loadChannel()
  assert.equal(data.state.channel.value, 'prerelease')
  assert.deepEqual(await data.callbacks.setChannel('stable'), { ok: true })
  assert.deepEqual(saved, [false]); assert.equal(data.state.channel.value, 'stable')
  assert.deepEqual(await data.callbacks.setChannel('prerelease'), { ok: true }); assert.deepEqual(saved, [false, true])
  // a refusal leaves the shown choice and says so
  const refused = make({}, { channel: { get: async () => false, set: async () => { throw new Error('native side down') } } })
  assert.deepEqual(await refused.callbacks.setChannel('prerelease'), { ok: false }); assert.equal(refused.state.channel.value, 'stable')
  // no native side (the Web page of the host): nothing is claimed
  assert.deepEqual(await make({}).callbacks.setChannel('prerelease'), { ok: false })
  await make({}).callbacks.loadChannel()
})

test('the funnel of the guide goes to the product, which hands it to its event sender', () => {
  const sent = []
  const data = make({}, { funnel: (props) => sent.push(props) })
  data.callbacks.funnel({ phase: 'reached', step: 'intro' })
  assert.deepEqual(sent, [{ phase: 'reached', step: 'intro' }])
  assert.doesNotThrow(() => make({}).callbacks.funnel({ phase: 'reached', step: 'intro' }), 'a product without a sender is fine')
})
