import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'

// The places of the Panel that show a game, a client, an address or a device are boxes of the screenshot redaction (data-redact); tools/redact-sentinel of foundation-ui sweeps the real pages for the rest.
const read = (p) => fs.readFileSync(new URL(p, import.meta.url), 'utf8')

test('the big screen: the names of the games (grid, list, recent strip, the running game) are marked', () => {
  assert.match(read('./components/AppGridView.vue'), /class="tile-name" data-redact="text"/)
  assert.match(read('./components/AppListView.vue'), /class="list-name" data-redact="text"/)
  assert.match(read('./components/AppRecentStrip.vue'), /class="recent-name" data-redact="text"/)
  assert.match(read('./components/RunningGameBar.vue'), /class="running-name" data-redact="text"/)
})
test('the host performance window marks the client and the address of the session', () => {
  const tool = read('../tool-window/tools/HostPerformanceTool.vue')
  assert.match(tool, /<b data-redact="client">\{\{ sessionTitle \}\}<\/b>/)
  assert.match(tool, /<span data-redact="address">\{\{ sessionMeta \}\}<\/span>/)
})
test('the USB page and the live session page mark the host, the remote devices and the client', () => {
  const usb = read('../components/controllersHub/UsbPassthroughPanel.vue')
  assert.match(usb, /class="m-usb__host" data-redact="address"/)
  assert.ok((usb.match(/data-redact="(?:address|serial)"/g) || []).length >= 3)
  const live = read('../components/m3e/LiveSessionPage.vue')
  assert.ok((live.match(/data-redact="client"/g) || []).length >= 2)
})
