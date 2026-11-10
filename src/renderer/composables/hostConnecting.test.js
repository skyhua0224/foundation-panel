import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'
import { createProxyGate, FIRST_ROUND_DELAYS } from './proxyGate.js'

const read = (rel) => fs.readFileSync(new URL(rel, import.meta.url), 'utf8')

test('the panel opens in the connecting state and probes longer while it is connecting', () => {
  const shell = read('../components/SidebarMenu.vue')
  assert.match(shell, /useHostConnection\(\{\s*[^}]*?starting: true,\s*probe: probeHost,/)
  assert.match(shell, /hostData\.probe\(hostConn\.state\.status === 'connecting' \? 6000 : 2500\)/)
  // the neutral page is the offline component's connecting variant (foundation-ui), not the error one
  assert.match(shell, /<MHostOffline v-if="offlineShown"/)
})

test('the proxy gate is quiet for about 25 s of tries, longer than a cold start needs, and still says so when the proxy is really blocked', async () => {
  const waits = FIRST_ROUND_DELAYS.reduce((a, b) => a + b, 0)
  assert.ok(waits >= 20000 && waits <= 30000, `${waits} ms of waiting in the first round`)
  assert.equal(FIRST_ROUND_DELAYS[0], 0, 'the first try is at once')
  const seen = []
  let calls = 0
  const gate = createProxyGate({ verify: async () => { calls += 1; return false }, onShown: () => { seen.push(`shown after ${calls}`); gate.stop() }, sleep: () => Promise.resolve() })
  await gate.run()
  assert.deepEqual(seen, [`shown after ${FIRST_ROUND_DELAYS.length}`], 'a proxy that never answers is shown only after the whole first round')
})

test('the shell reads the host through hostData only (the module the dedicated host channel will replace), and draws the kept configuration at once', () => {
  const shell = read('../components/SidebarMenu.vue')
  assert.doesNotMatch(shell, /sunshine\.probeHost\(/); assert.doesNotMatch(shell, /fetch\(`\$\{base\}\/api\/config`\)/)
  assert.match(shell, /const keptHostConfig = hostData\.peek\('config'\)\nif \(keptHostConfig\) hostConfig\.value = keptHostConfig/)
  assert.match(shell, /hostData\.refreshConfig\(\)/)
  const module = read('./hostData.js')
  assert.match(module, /probe: \(timeoutMs\) => sunshine\.probeHost\(timeoutMs\)/); assert.match(module, /peek\(topic\)/)
})
