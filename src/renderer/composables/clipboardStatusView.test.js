import test from 'node:test'
import assert from 'node:assert/strict'
import { clipboardBundleFacts, clipboardStatusView, clipboardSyncState, clipboardSyncWorks } from './clipboardStatusView.js'

const texts = {
  never: 'never', secondsAgo: '{n} s ago', minutesAgo: '{n} min ago', running: 'running', notRunning: 'not running',
  agent: 'Agent', setting: 'Setting', settingOn: 'on', settingOff: 'off', settingUnknown: 'unknown', hostAgent: 'Host', hostSees: 'sees', hostDoesNotSee: 'does not see', hostUnknown: '?',
  heartbeat: 'Heartbeat', heartbeatError: 'Heartbeat failure', lastError: 'Last failure', capabilityError: 'Setting not readable', lastConnected: 'Connected',
  states: { connected: 'Connected', degraded: 'Degraded', disconnected: 'Disconnected', connecting: 'Connecting', disabled: 'Off in the host', off: 'Not running', unknown: 'Unknown' },
  errors: { connect_refused: 'refused', timeout: 'timeout', tls: 'tls', http_401: '401', http_403: '403', http_5xx: '5xx', stream_ended: 'ended', credential_missing: 'no credential', heartbeat: 'heartbeat', other: 'other' },
}
const base = { agent_active: true, service_allowed: true, transport_state: 'connected', last_error_class: 'none', heartbeat_error: 'none', capability_error: 'none', host_gui_alive: true, last_heartbeat_ok_ms: 1_000_000 - 5000 }

test('connected only when the agent runs, the stream is open and the host says its setting is on', () => {
  assert.equal(clipboardSyncState(base), 'connected')
  assert.equal(clipboardSyncWorks(base), true)
})

test('a host setting that could not be read is unknown, not allowed', () => {
  assert.equal(clipboardSyncState({ ...base, service_allowed: null, capability_error: 'timeout' }), 'unknown')
  assert.equal(clipboardSyncWorks({ ...base, service_allowed: null }), false)
})

test('an open stream the host stopped hearing is degraded and is not shown as working', () => {
  const degraded = { ...base, transport_state: 'degraded', heartbeat_error: 'timeout' }
  assert.equal(clipboardSyncState(degraded), 'degraded')
  assert.equal(clipboardSyncWorks(degraded), false)
  assert.equal(clipboardStatusView(degraded, texts, 1_000_000).tone, 'warn')
})

test('the other states keep their order of precedence', () => {
  assert.equal(clipboardSyncState(null), 'unknown')
  assert.equal(clipboardSyncState({ ...base, agent_active: false }), 'off')
  assert.equal(clipboardSyncState({ ...base, transport_state: 'stopped' }), 'off')
  assert.equal(clipboardSyncState({ ...base, service_allowed: false }), 'disabled')
  assert.equal(clipboardSyncState({ ...base, transport_state: 'connecting' }), 'connecting')
  assert.equal(clipboardSyncState({ ...base, transport_state: 'disconnected' }), 'disconnected')
})

test('the rows say what the host sees and when it last heard the agent', () => {
  const view = clipboardStatusView({ ...base, host_gui_alive: false, last_error_class: 'http_403' }, texts, 1_000_000)
  const by = Object.fromEntries(view.rows.map((r) => [r.id, r.text]))
  assert.equal(by.host, 'does not see')
  assert.equal(by.heartbeat, '5 s ago')
  assert.equal(by.lastError, '403')
  assert.ok(!('heartbeatError' in by))
})

test('a failure word the page does not know is shown as other and never as the raw text', () => {
  const view = clipboardStatusView({ ...base, last_error_class: 'C:\\Users\\someone\\secret' }, texts, 1_000_000)
  assert.equal(view.rows.find((r) => r.id === 'lastError').text, 'other')
})

test('the bundle carries words, counts and times and never the text of the last error', () => {
  const facts = clipboardBundleFacts({ ...base, last_error: 'error sending request for url (https://127.0.0.1:47990/x)' })
  assert.ok(!('last_error' in facts))
  assert.equal(facts.state, 'connected')
  assert.equal(clipboardBundleFacts(null), null)
})
