import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'

const shell = fs.readFileSync(new URL('./SidebarMenu.vue', import.meta.url), 'utf8')
const diag = fs.readFileSync(new URL('./m3e/DiagnosticsPage.vue', import.meta.url), 'utf8')

test('the panel mounts the one report surface, and its reports carry the last host log lines', () => {
  assert.match(shell, /<MErrorHost[^\n]*:open-url="actions_openExternal"/, 'the GitHub way out opens in the browser of the person')
  assert.match(shell, /const refreshLogs = async \(\) => \{[^\n]*\/api\/logs/, 'the shell reads the host log')
  assert.match(shell, /logs: String\(recentLogText\.value \|\| ''\)\.split\('\\n'\)\.slice\(-40\)\.join\('\\n'\)/, 'forty lines go into the report context (buildReport scrubs them)')
  assert.match(shell, /startAutoReport\(\{ context: errorContext \}\); void refreshLogs\(\)/)
})

test('the diagnostics page is the shared one, so its feedback row sends through the same surface', () => {
  assert.match(diag, /<MDiagnostics\b/); assert.doesNotMatch(diag, /issues\/new/, 'no second, private report path')
})
