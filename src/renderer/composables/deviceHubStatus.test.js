import test from 'node:test'
import assert from 'node:assert/strict'
import {
  canTestMicrophone,
  microphoneOverviewState,
  microphoneStatusTone,
  microphoneUsesUsbip,
} from './deviceHubStatus.js'

test('microphone status prioritizes faults over an old online flag', () => {
  const usb = { configured_backend: 'usbip_experimental', component_available: true }
  assert.equal(microphoneStatusTone({ ...usb, state: 'device_faulted', online: true }), 'state-error')
  assert.equal(microphoneStatusTone({ ...usb, error_code: 'MIC_FAILURE', device_created: true }), 'state-error')
  assert.equal(microphoneStatusTone({ ...usb, device_created: true }), 'state-ready')
  assert.equal(microphoneStatusTone({ state: 'absent' }), '')
})

test('microphone test requires an available component and enabled backend', () => {
  assert.equal(canTestMicrophone({ component_available: true, configured_backend: 'usbip_experimental' }), true)
  assert.equal(canTestMicrophone({ component_available: true, configured_backend: 'disabled' }), false)
  assert.equal(canTestMicrophone({ component_available: false, configured_backend: 'usbip_experimental' }), false)
  assert.equal(canTestMicrophone({ component_available: true, configured_backend: 'usbip_experimental' }, true), false)
})

test('microphone overview distinguishes missing, waiting, idle and capturing', () => {
  const usb = { configured_backend: 'usbip_experimental' }
  assert.equal(microphoneOverviewState({}), 'unknown')
  assert.equal(microphoneOverviewState(usb), 'missing')
  assert.equal(microphoneOverviewState({ ...usb, component_available: true }), 'waiting')
  assert.equal(microphoneOverviewState({ ...usb, component_available: true, device_created: true }), 'idle')
  assert.equal(microphoneOverviewState({ ...usb, component_available: true, device_created: true, host_streaming: true }), 'capturing')
})

test('VB-Cable and automatic fallback remain testable without the experimental component', () => {
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
