import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { driverGuardApi } from './useDriverGuard.js'

const here = path.dirname(fileURLToPath(import.meta.url))
const base = async () => 'http://127.0.0.1:47990'

test('reads and answers through the proxy of the host', async () => {
  const calls = []
  const fetcher = async (b, p, init) => { calls.push([p, init?.method || 'GET', init?.body]); return new Response(JSON.stringify({ v: 1, probe: 'done' }), { status: 200 }) }
  const api = driverGuardApi(base, fetcher)
  assert.deepEqual(await api.get(), { v: 1, probe: 'done' })
  await api.post({ action: 'retry' })
  assert.deepEqual(calls[0], ['/api/gpu-driver-guard', 'GET', undefined])
  assert.deepEqual(calls[1], ['/api/gpu-driver-guard', 'POST', '{"action":"retry"}'])
})

test('a status other than 2xx is an error (an older host has no route)', async () => {
  const api = driverGuardApi(base, async () => new Response('{}', { status: 404 }))
  await assert.rejects(api.get(), (e) => e.status === 404)
})

test('the overview draws the shared notice, opens links in the browser and answers it', () => {
  const home = fs.readFileSync(path.join(here, '../components/PanelHome.vue'), 'utf8')
  assert.match(home, /<MDriverGuardNotice[^>]*:guard="driverGuard"[^>]*:open="openExternalUrl"/)
  assert.match(home, /@dismiss="dismissDriver"/)
  assert.match(home, /@retry="retryProbe"/)
})
