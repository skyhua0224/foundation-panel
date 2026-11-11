import assert from 'node:assert/strict'
import fs from 'node:fs'
import test from 'node:test'

// 开发者 pages (owner 2026-11-11): the simulation of the host that is away is real for all three kinds, ends cleanly, and shows in the header of every developer page; the answer about telemetry reaches 测试工具 as known or not yet known.
const read = (p) => fs.readFileSync(new URL(`../${p}`, import.meta.url), 'utf8')
const menu = read('components/SidebarMenu.vue')
const pages = read('components/m3e/DeveloperPages.vue')

test('「无响应」 is simulated for real: the offline surface of a host that runs and does not answer', () => {
  assert.match(menu, /hostSimulated\.value === 'offline' \|\| hostSimulated\.value === 'unresponsive' \|\| proxyBlock\.blocked/, 'the content gives way to the status page')
  assert.match(menu, /hostSimulated\.value === 'unresponsive' \? 'unresponsive'/, 'the cause is the unresponsive one')
  assert.match(menu, /hostSimulated\.value === 'unresponsive' \? \{ \.\.\.hostConn\.state, status: 'offline', verdict: 'unresponsive'/)
  assert.match(menu, /hostSimulated\.value = kind === 'offline' \|\| kind === 'unresponsive' \? kind : ''/)
})
test('ending or changing a simulation puts the panel back: the error surface of a simulated stop goes, a real error does not', () => {
  assert.match(menu, /const wasStopped = hostStopSimulated\s+hostStopSimulated = kind === 'stopped'/)
  assert.match(menu, /else if \(wasStopped \|\| !kind\) useErrors\(\)\.dismissError\(\)/)
  assert.match(menu, /if \(kind\) hostSimulatedTimer = setTimeout\(\(\) => simulateHost\(''\), 60000\)/)
})
test('the offline page of a simulation carries the chip and the way to end it; every developer page has it in its header', () => {
  assert.match(menu, /<div v-if="offlineShown" class="panel-sim-end"><DevSimChip :locale="locale" \/><\/div>/)
  assert.match(pages, /<template #actions><DevSimChip :locale="locale" \/><\/template>/)
})
test('测试工具 is told whether the answer about telemetry is known', () => {
  assert.match(menu, /:consent="telemetry\.consent\.value" :consent-ready="telemetry\.loaded\.value"/)
  assert.match(pages, /:consent-on="consent === 'on'" :consent-ready="consentReady"/)
  assert.match(pages, /consentReady: \{ type: Boolean, default: true \}/)
})
