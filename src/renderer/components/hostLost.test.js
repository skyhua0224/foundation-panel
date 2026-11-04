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

test('a host that runs and does not answer opens 「Sunshine 没有响应」 and the dialog closes when it answers again', () => {
  assert.match(shell, /onHung: \(\{ at, seconds \}\) => reportHostHung\(\{ at, seconds \}\)/)
  assert.match(shell, /onHungEnd: \(\) => dismissHostHung\(\)/)
  assert.match(shell, /reportHostStop, reportHostHung, dismissHostHung,/, 'both come from the shared package')
  assert.match(shell, /onLost: \(\{ at \}\) => reportHostStop\(\{ at \}\)/, 'a stop is still opened only on the facts')
})

test('the facts carry the process id of the service wrapper, so that a restart of the service is told from a crash', () => {
  const rust = fs.readFileSync(new URL('../../../src-tauri/src/host_facts.rs', import.meta.url), 'utf8')
  assert.match(rust, /pub service_pid: u32/)
  assert.match(rust, /status\.dwProcessId/)
  assert.match(rust, /let \(service, service_pid\) = service_state\(\)/)
})
