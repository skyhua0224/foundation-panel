import assert from 'node:assert/strict'
import fs from 'node:fs'
import test from 'node:test'

// The overview's device tiles get their dot from the shared state mapping (foundation-ui `deviceTiles.js`, tested in the package and in the Web UI);
// the panel must feed it the runtime state and nothing else: no constant that makes a tile ready.
const home = fs.readFileSync(new URL('../components/PanelHome.vue', import.meta.url), 'utf8')

test('the panel overview derives the microphone and gamepad dots from the runtime state', () => {
  assert.match(home, /ready: padIsReady\(props\.runtime\.ds\?\.state\)/)
  assert.match(home, /ready: micIsReady\(\{ state: props\.runtime\.mic\?\.state \}\)/)
  assert.ok(!/ready: true/.test(home), 'no tile is ready by default')
})
