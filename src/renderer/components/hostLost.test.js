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

test('the connection reads the facts of the operating system, so that a silence of the host is not announced as a stop', () => {
  assert.match(shell, /facts: \(\) => tools\.getHostFacts\(\)/)
  assert.match(shell, /facts: \(\) => tools\.getHostFacts\(\),\s*\n\s*onLost: \(\{ at \}\) => reportHostStop\(\{ at \}\)/, 'the facts come before the stop surface in the options')
  const adapter = fs.readFileSync(new URL('../tauri-adapter.js', import.meta.url), 'utf8')
  assert.match(adapter, /getHostFacts: \(\) => invoke\('get_sunshine_host_facts'\)/)
})
