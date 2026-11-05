import assert from 'node:assert/strict'
import test from 'node:test'
import { readFileSync } from 'node:fs'

const root = new URL('../../../', import.meta.url)
const read = (path) => readFileSync(new URL(path, root), 'utf8')

test('native About, pairing, and log entries target the main shell pages', () => {
  const rust = read('src-tauri/src/windows.rs')
  const parts = read('src/renderer/composables/panelParts.js')
  const diagnostics = read('src/renderer/components/m3e/DiagnosticsPage.vue')
  const commands = read('src-tauri/src/commands.rs')
  const toolbar = read('src-tauri/src/toolbar.rs')
  assert.doesNotMatch(rust, /(?:about|console|pin)\/index\.html/)
  assert.match(parts, /about: \{ route: 'about'/)
  assert.match(parts, /pairing: \{ route: 'web-clients'/)
  assert.match(parts, /logs: \{ route: 'diagnostics'/)
  assert.match(parts, /pairing: '\/pin#pairing'/)
  assert.match(parts, /logs: '\/troubleshooting'/)
  for (const operation of ['refreshLogs', 'exportLogs', 'clearLogs']) assert.match(diagnostics, new RegExp(`function ${operation}`))
  assert.match(commands, /"nr" => windows::open_main_panel_page\(&app, "quality"\)/)
  assert.match(commands, /"performance" \| "host_performance" => windows::open_main_panel_page\(&app, "live-session"\)\?/)
  assert.match(toolbar, /"performance" \| "toolbar_performance" => open_panel_page\(app, "live-session"\)/)
  assert.match(toolbar, /"rtss" \| "toolbar_rtss" => open_panel_page\(app, "rtss"\)/)
})
