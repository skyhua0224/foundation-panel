import assert from 'node:assert/strict'
import test from 'node:test'
import { readFileSync } from 'node:fs'

const root = new URL('../../../', import.meta.url)
const read = (path) => readFileSync(new URL(path, root), 'utf8')

test('native About, pairing, and log entries target the main shell pages', () => {
  const rust = read('src-tauri/src/windows.rs')
  const parts = read('src/renderer/composables/panelParts.js')
  const diagnostics = read('src/renderer/components/m3e/DiagnosticsPage.vue')
  assert.doesNotMatch(rust, /(?:about|console|pin)\/index\.html/)
  assert.match(parts, /about: \{ route: 'about'/)
  assert.match(parts, /pairing: \{ route: 'web-clients'/)
  assert.match(parts, /logs: \{ route: 'diagnostics'/)
  assert.match(parts, /pairing: '\/pin#pairing'/)
  assert.match(parts, /logs: '\/troubleshooting'/)
  for (const operation of ['refreshLogs', 'exportLogs', 'clearLogs']) assert.match(diagnostics, new RegExp(`function ${operation}`))
})
