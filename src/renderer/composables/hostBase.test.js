import assert from 'node:assert/strict'
import fs from 'node:fs'
import test from 'node:test'
import { hostBase, hostFetch } from './hostBase.js'

test('the known address is used as it is, without a trailing slash', async () => {
  assert.equal(await hostBase(() => 'http://127.0.0.1:48081/', () => { throw new Error('not asked') }), 'http://127.0.0.1:48081')
  assert.equal(await hostBase('http://127.0.0.1:48081', null), 'http://127.0.0.1:48081')
})
test('an address that is not known yet is asked for, not guessed', async () => {
  let asked = 0
  assert.equal(await hostBase(() => '', async () => { asked += 1; return 'http://127.0.0.1:48081/' }), 'http://127.0.0.1:48081')
  assert.equal(asked, 1)
})
test('no address at all is an error, never an empty prefix', async () => {
  await assert.rejects(hostBase(() => '', async () => ''), /not known yet/)
  await assert.rejects(hostBase('', undefined), /not known yet/)
})
test('a request goes to the host through the proxy, and not at all when the address is missing', async () => {
  const seen = []
  const realFetch = globalThis.fetch
  globalThis.fetch = async (url, init) => { seen.push([url, init?.method]); return { ok: true } }
  try {
    await hostFetch(() => hostBase('http://h:1', null), '/api/config', { method: 'GET' })
    assert.deepEqual(seen, [['http://h:1/api/config', 'GET']])
    await assert.rejects(hostFetch(() => hostBase('', async () => ''), '/api/system/info'), /not known yet/)
    assert.equal(seen.length, 1, 'nothing was requested without an address')
  } finally { globalThis.fetch = realFetch }
})
test('the overview asks the host only through the address: no request with the raw reactive prefix is left', () => {
  const home = fs.readFileSync(new URL('../components/PanelHome.vue', import.meta.url), 'utf8')
  assert.ok(!/fetch\(`\$\{proxyUrl\.value\}\$\{path\}`/.test(home), 'a fetch with an empty prefix goes to the panel itself')
  assert.match(home, /hostFetch\(base, path/)
})
