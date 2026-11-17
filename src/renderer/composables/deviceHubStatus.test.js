import test from 'node:test'
import assert from 'node:assert/strict'
import {
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
  VB_CABLE_PAGE,
  VB_CABLE_STATES,
  vbCableErrorKind,
  vbCableJob,
  vbCableNeedsEnable,
  vbCableNeedsInstall,
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

test('VB-CABLE: the download is offered when it is not installed or not usable, never before the host has answered', () => {
  assert.equal(vbCableNeedsInstall('ready'), false)
  assert.equal(vbCableNeedsInstall('unknown'), false)
  for (const state of ['partial', 'unavailable', 'missing']) assert.equal(vbCableNeedsInstall(state), true, state)
  assert.match(VB_CABLE_PAGE, /^https:\/\/vb-audio\.com\//)
})

test('VB-CABLE: installed but disabled offers the Device Manager, not a download', () => {
  assert.equal(vbCableNeedsEnable('disabled'), true)
  assert.equal(vbCableNeedsInstall('disabled'), false)
  for (const state of ['ready', 'partial', 'unavailable', 'missing', 'unknown']) assert.equal(vbCableNeedsEnable(state), false, state)
  assert.equal(vbCableState({ vb_cable: { state: 'disabled', installed: true } }), 'disabled')
})

test('VB-CABLE: the job of the host is read as it is reported, an older host has none', () => {
  assert.deepEqual(vbCableJob(null), { running: false, step: '', failed: false, errorCode: '' })
  assert.deepEqual(vbCableJob({ vb_cable: { state: 'missing' } }), { running: false, step: '', failed: false, errorCode: '' })
  assert.deepEqual(vbCableJob({ vb_cable: { job: { action: 'install', step: 'verifying', running: true } } }), { running: true, step: 'verifying', failed: false, errorCode: '' })
  assert.equal(vbCableJob({ vb_cable: { job: { step: 'something-new', running: true } } }).step, 'downloading')
  const failed = vbCableJob({ vb_cable: { job: { action: 'enable', step: 'failed', running: false, error_code: 'VB_NEEDS_ADMIN' } } })
  assert.equal(failed.failed, true)
  assert.equal(failed.errorCode, 'VB_NEEDS_ADMIN')
})

test('VB-CABLE: a failed job is told by what a person can do about it', () => {
  assert.equal(vbCableErrorKind('VB_DOWNLOAD_FAILED'), 'download')
  assert.equal(vbCableErrorKind('VB_SIGNATURE_INVALID'), 'signature')
  assert.equal(vbCableErrorKind('VB_SIGNER_UNEXPECTED'), 'signature')
  assert.equal(vbCableErrorKind('VB_NEEDS_ADMIN'), 'admin')
  assert.equal(vbCableErrorKind('VB_ENABLE_FAILED'), 'failed')
  assert.equal(vbCableErrorKind(''), '')
})

for (const version_valid of [true, false]) {
  test(`external USB/IP is preserved with version_valid=${version_valid}`, () => {
    const status = { installed: true, externally_managed: true, version_valid, vhci_residual: true, error_code: 'USBIP-EXEC-006' }
    assert.equal(usbipTransportNeedsCleanup(status), false)
    assert.equal(usbipTransportNeedsCleanup({ ...status, externally_managed: false }), true)
  })
}
