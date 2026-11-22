import test from 'node:test'
import assert from 'node:assert/strict'
import {
  usbipCanReplaceLegacy,
  usbipResidualIsStale,
  usbipTransportNeedsCleanup,
  MICROPHONE_BACKENDS,
  canTestMicrophone,
  microphoneBufferedMs,
  microphoneComponentMissing,
  microphoneOverviewState,
  microphoneStatusTone,
  microphoneUsesUsbip,
  microphoneInputFresh,
  microphoneInputLevel,
  microphoneTestIsLive,
  VB_CABLE_STATES,
  vbCableNeedsEnable,
  vbCableState,
} from './deviceHubStatus.js'

const usb = { configured_backend: 'usbip_experimental', component_available: true, active_backend: 'usbip_experimental' }

test('microphone status prioritizes faults over an old online flag', () => {
  assert.equal(microphoneStatusTone({ ...usb, state: 'device_faulted', online: true }), 'state-error')
  assert.equal(microphoneStatusTone({ ...usb, state: 'idle', device_created: true }), 'state-ready')
  assert.equal(microphoneStatusTone({ configured_backend: 'usbip_experimental', error_code: 'MIC_USBIP_CREATE_FAILED' }), 'state-error')
  assert.equal(microphoneStatusTone({ state: 'absent' }), '')
})

test('microphone test needs an enabled backend and usbip-win2 for the USB microphone alone', () => {
  assert.equal(canTestMicrophone({ component_available: true, configured_backend: 'usbip_experimental' }), true)
  assert.equal(canTestMicrophone({ component_available: true, configured_backend: 'disabled' }), false)
  assert.equal(canTestMicrophone({ component_available: false, configured_backend: 'usbip_experimental' }), false)
  assert.equal(canTestMicrophone({ component_available: false, configured_backend: 'auto' }), true)
  assert.equal(canTestMicrophone({ component_available: true, configured_backend: 'usbip_experimental' }, true), false)
})

test('the USB microphone states map to the panel states', () => {
  const states = {
    creating: 'starting', enumerating: 'starting', idle: 'idle', absent: 'idle',
    host_capturing: 'capturing', remote_active: 'streaming', destroying: 'stopping', device_faulted: 'faulted',
  }
  for (const [host, panel] of Object.entries(states)) {
    assert.equal(microphoneOverviewState({ ...usb, state: host }), panel, host)
  }
  assert.equal(microphoneOverviewState({}), 'unknown')
})

test('a missing usbip-win2 reads as missing for the USB microphone and as waiting for automatic', () => {
  assert.equal(microphoneOverviewState({ configured_backend: 'usbip_experimental' }), 'missing')
  assert.equal(microphoneOverviewState({ configured_backend: 'usbip_experimental', error_code: 'MIC_USBIP_COMPONENT_UNAVAILABLE' }), 'missing')
  assert.equal(microphoneOverviewState({ configured_backend: 'auto' }), 'waiting')
  assert.equal(microphoneComponentMissing({ configured_backend: 'auto', component_available: false }), true)
  assert.equal(microphoneComponentMissing({ configured_backend: 'usbip_experimental', component_available: true }), false)
  assert.equal(microphoneComponentMissing({ configured_backend: 'vb_cable', component_available: false }), false)
  assert.equal(microphoneComponentMissing({ configured_backend: 'auto', state: 'unsupported' }), false)
})

test('buffered audio is shown in milliseconds', () => {
  assert.equal(microphoneBufferedMs({ buffered_bytes: 96 * 30 }), 30)
  assert.equal(microphoneBufferedMs({}), 0)
})

test('the four backend values are the host values', () => {
  assert.deepEqual([...MICROPHONE_BACKENDS], ['auto', 'vb_cable', 'usbip_experimental', 'disabled'])
})

test('VB-Cable and automatic fallback remain testable without usbip-win2', () => {
  for (const backend of ['vb_cable', 'auto']) {
    const status = { configured_backend: backend, component_available: false }
    assert.equal(canTestMicrophone(status), true)
    assert.equal(microphoneOverviewState(status), 'waiting')
    assert.equal(canTestMicrophone(status, true), false)
    const active = { ...status, active_backend: 'vb_cable', device_created: true, error_code: 'STALE_USBIP_ERROR' }
    assert.equal(microphoneOverviewState(active), 'active')
    assert.equal(microphoneUsesUsbip(active), false)
  }
  assert.equal(microphoneOverviewState({ configured_backend: 'disabled', device_created: true }), 'disabled')
  assert.equal(canTestMicrophone({ configured_backend: 'vb_cable', state: 'unsupported' }), false)
})

test('the live client microphone level is clamped and its freshness has a bounded window', () => {
  assert.equal(microphoneInputLevel({ input_peak: 0.42 }), 0.42)
  assert.equal(microphoneInputLevel({ input_peak: 4 }), 1)
  assert.equal(microphoneInputLevel({ input_peak: -1 }), 0)
  assert.equal(microphoneInputLevel({}), 0)
  assert.equal(microphoneInputLevel({ input_peak: 'broken' }), 0)
  assert.equal(microphoneInputFresh({ input_active_ms: 250 }), true)
  assert.equal(microphoneInputFresh({ input_active_ms: 2999 }), true)
  assert.equal(microphoneInputFresh({ input_active_ms: 3000 }), false)
  assert.equal(microphoneInputFresh({ input_active_ms: -1 }), false)
  assert.equal(microphoneInputFresh({}), false)
})

test('a live test answer is recognized regardless of its backend', () => {
  assert.equal(microphoneTestIsLive({ mode: 'live', backend: 'vb_cable', success: true }), true)
  assert.equal(microphoneTestIsLive({ mode: 'tone', backend: 'usbip_experimental', success: true }), false)
  assert.equal(microphoneTestIsLive({}), false)
})

test('VB-CABLE: the state of the host answer, `unknown` before it and for a state this version does not know', () => {
  for (const state of VB_CABLE_STATES) assert.equal(vbCableState({ success: true, vb_cable: { state } }), state)
  assert.equal(vbCableState(null), 'unknown')
  assert.equal(vbCableState({}), 'unknown')
  assert.equal(vbCableState({ vb_cable: { state: 'unsupported' } }), 'unknown')
  assert.equal(vbCableState({ vb_cable: { state: 'something-new' } }), 'unknown')
})

test('VB-CABLE: installed but disabled offers the Device Manager, not a download', () => {
  assert.equal(vbCableNeedsEnable('disabled'), true)
  for (const state of ['ready', 'partial', 'unavailable', 'missing', 'unknown']) assert.equal(vbCableNeedsEnable(state), false, state)
  assert.equal(vbCableState({ vb_cable: { state: 'disabled', installed: true } }), 'disabled')
})

for (const version_valid of [true, false]) {
  test(`external USB/IP is preserved with version_valid=${version_valid}`, () => {
    const status = { installed: true, externally_managed: true, version_valid, vhci_residual: true, error_code: 'USBIP-EXEC-006' }
    assert.equal(usbipTransportNeedsCleanup(status), false)
    assert.equal(usbipTransportNeedsCleanup({ ...status, externally_managed: false }), true)
  })
}

test('a working controller with older ones beside it is stale, anything else needs the full cleanup', () => {
  const base = { installed: true, version_valid: true, vhci_residual: true }
  assert.equal(usbipResidualIsStale({ ...base, residual_kind: 'stale' }), true)
  assert.equal(usbipResidualIsStale({ ...base, residual_kind: 'broken' }), false)
  assert.equal(usbipResidualIsStale({ ...base, residual_kind: '' }), false)
  assert.equal(usbipResidualIsStale({ ...base, vhci_residual: false, residual_kind: 'stale' }), false)
  // an installation the person manages is not uninstalled, but its duplicate controller may go (one stays live)
  assert.equal(usbipResidualIsStale({ ...base, externally_managed: true, residual_kind: 'stale' }), true)
  assert.equal(usbipTransportNeedsCleanup({ ...base, externally_managed: true, residual_kind: 'stale' }), false)
  assert.equal(usbipResidualIsStale(), false)
  // the stale state still counts as needing a cleanup for the summary that only asks "is there a problem"
  assert.equal(usbipTransportNeedsCleanup({ ...base, residual_kind: 'stale' }), true)
})

test('the replacement of an external USB/IP is offered only when the host says it may be replaced', () => {
  assert.equal(usbipCanReplaceLegacy({ externally_managed: true, replaceable: true }), true)
  assert.equal(usbipCanReplaceLegacy({ externally_managed: true, replaceable: false }), false)
  assert.equal(usbipCanReplaceLegacy({ externally_managed: true }), false)
  // the host's own package is never offered for replacement
  assert.equal(usbipCanReplaceLegacy({ externally_managed: false, replaceable: true }), false)
  assert.equal(usbipCanReplaceLegacy(), false)
})
