import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'

const shell = fs.readFileSync(new URL('./SidebarMenu.vue', import.meta.url), 'utf8')

test('the panel shows the 「Sunshine 意外停止」 surface when the host stops without being asked, and starts the automatic report', () => {
  assert.match(shell, /onLost: \(\{ at \}\) => reportHostStop\(\{ at \}\)/)
  assert.match(shell, /startAutoReport\(\{ context: errorContext \}\)/)
  assert.match(shell, /setErrorContext\(errorContext\)/)
  assert.match(shell, /<MErrorHost[^>]*:restart="\(\) => hostConn\.restart\('service'\)"/, 'the restart of a host that does not answer is the way out')
})
