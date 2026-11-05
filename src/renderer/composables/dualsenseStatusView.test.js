import test from 'node:test'
import assert from 'node:assert/strict'
import { statusRows, statusTone } from './dualsenseStatusView.js'

const say = (path, params = {}) => `${path}${Object.keys(params).length ? JSON.stringify(params) : ''}`

const ready = () => ({
  state: 'ready',
  host: { reachable: true, tuning_available: true, native_compiled: true, native_active: false, active_controllers: 0 },
  transport: {
    supported: true, installed: true, version: '0.9.8.1', pinned_version: '0.9.8.1',
    version_supported: true, ready: true, signature: 'signed', signer: 'Vadim Grn',
  },
})

const rowsOf = (status) => Object.fromEntries(statusRows(status, say).map((row) => [row.id, row]))

test('a complete setup reads as built in, available, signed and idle', () => {
  const rows = rowsOf(ready())
  assert.deepEqual(rows.native, { id: 'native', tone: 'ok', text: 'nativeBuilt' })
  assert.equal(rows.transport.text, 'transportReady{"version":"0.9.8.1"}')
  assert.equal(rows.signature.text, 'signedBy{"signer":"Vadim Grn"}')
  assert.equal(rows.controller.tone, 'neutral')
  assert.equal(rows.controller.text, 'controllerIdle')
})

test('a connected controller is reported with its count', () => {
  const status = ready()
  status.host.active_controllers = 2
  assert.equal(rowsOf(status).controller.text, 'controllerActive{"count":2}')
  status.host.active_controllers = 0
  status.host.native_active = true
  assert.equal(rowsOf(status).controller.text, 'controllerActive{"count":1}')
})

test('a missing transport is a warning and leaves the signature unchecked', () => {
  const status = ready()
  status.transport = { supported: true, installed: false, pinned_version: '0.9.8.1' }
  const rows = rowsOf(status)
  assert.deepEqual([rows.transport.tone, rows.transport.text], ['warn', 'notInstalled'])
  assert.equal(rows.signature.text, 'unavailable')
})

test('another version and a bad signature are errors, an unreadable one is not', () => {
  const wrong = ready()
  wrong.transport.version = '0.9.9.0'
  wrong.transport.version_supported = false
  assert.deepEqual(
    [rowsOf(wrong).transport.tone, rowsOf(wrong).transport.text],
    ['bad', 'mismatch{"version":"0.9.9.0","pinned":"0.9.8.1"}'],
  )
  for (const signature of ['unsigned', 'invalid']) {
    const status = ready()
    status.transport.signature = signature
    assert.equal(rowsOf(status).signature.tone, 'bad', signature)
  }
  const unknown = ready()
  unknown.transport.signature = 'unknown'
  assert.equal(rowsOf(unknown).signature.tone, 'neutral')
})

test('an old or unreachable host shows unknown instead of guessing', () => {
  const old = ready()
  old.host = { reachable: true, tuning_available: false }
  assert.equal(rowsOf(old).native.text, 'nativeUnknown')
  assert.equal(rowsOf(old).controller.text, 'unavailable')
  assert.deepEqual(statusRows(null, say), [])
})

test('tones follow how actionable the state is', () => {
  assert.equal(statusTone('ready'), 'ok')
  assert.equal(statusTone('active'), 'ok')
  assert.equal(statusTone('usbip_missing'), 'warn')
  assert.equal(statusTone('host_unsupported'), 'warn')
  assert.equal(statusTone('usbip_unsigned'), 'bad')
  assert.equal(statusTone('usbip_version'), 'bad')
  assert.equal(statusTone('loading'), 'neutral')
})
