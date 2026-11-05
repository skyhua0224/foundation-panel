import test from 'node:test'
import assert from 'node:assert/strict'
import { nrSessionRequest, nrResetRequest } from './nrSessionControls.js'
const active = {
  id: 42,
  nr_toggle_supported: true,
  nr_live_controls_version: 3,
  nr_requested_enabled: true,
  nr_state: 'active',
  nr_requested_scale_percent: 75,
  nr_scale_percent: 75
}
test('live controls cannot redirect an edit or write while a previous setting is pending', () => {
  assert.throws(
    () => nrSessionRequest(active, 43, { scalePercent: 50 }),
    /nr_session_ended/
  )
  assert.throws(
    () =>
      nrSessionRequest({ ...active, nr_requested_scale_percent: 50 }, 42, {
        enabled: false
      }),
    /nr_settings_pending/
  )
  assert.throws(
    () => nrSessionRequest(active, 42, { enabled: false }, false),
    /nr_toggle_unsupported/
  )
})
test('scale requires a compatible enabled stream and the 5 percent grid', () => {
  assert.throws(() =>
    nrSessionRequest({ ...active, nr_live_controls_version: 1 }, 42, {
      scalePercent: 50
    })
  )
  assert.throws(
    () => nrSessionRequest(active, 42, { scalePercent: 67 }),
    /nr_request_invalid/
  )
  assert.deepEqual(nrSessionRequest(active, 42, { scalePercent: 20 }), {
    id: 42,
    enabled: true,
    scalePercent: 20
  })
  assert.deepEqual(
    nrSessionRequest(
      { ...active, nr_requested_enabled: false, nr_state: 'disabled' },
      42,
      { enabled: true }
    ),
    { id: 42, enabled: true }
  )
})

test('unadvertised scale support and disabled streams reject scale writes', () => {
  assert.throws(
    () =>
      nrSessionRequest({ ...active, nr_live_controls_version: undefined }, 42, {
        scalePercent: 50
      }),
    /nr_toggle_unsupported/
  )
  assert.throws(
    () =>
      nrSessionRequest(
        { ...active, nr_requested_enabled: false, nr_state: 'disabled' },
        42,
        { scalePercent: 50 }
      ),
    /nr_toggle_unsupported/
  )
})

test('the options of the overlay pass with their controls version and their ranges', () => {
  assert.deepEqual(nrSessionRequest(active, 42, { intensity: 0.55 }), { id: 42, enabled: true, intensity: 0.55 })
  assert.deepEqual(nrSessionRequest(active, 42, { style: 4, skinStructureStrength: 0.3, autoMask: true }), { id: 42, enabled: true, style: 4, skinStructureStrength: 0.3, autoMask: true })
  assert.deepEqual(nrSessionRequest(active, 42, { uiCorrection: true, motionQuality: 3 }), { id: 42, enabled: true, uiCorrection: true, motionQuality: 3 })
  // options are kept for the next start while the enhancement is off, as the overlay sent them
  assert.deepEqual(nrSessionRequest({ ...active, nr_requested_enabled: false, nr_state: 'disabled' }, 42, { intensity: 1 }), { id: 42, enabled: false, intensity: 1 })
  assert.throws(() => nrSessionRequest(active, 42, { intensity: 1.2 }), /nr_request_invalid/)
  assert.throws(() => nrSessionRequest(active, 42, { motionQuality: 4 }), /nr_request_invalid/)
  assert.throws(() => nrSessionRequest(active, 42, { style: 1.5 }), /nr_request_invalid/)
  assert.throws(() => nrSessionRequest({ ...active, nr_live_controls_version: 2 }, 42, { style: 1 }), /nr_toggle_unsupported/)
  assert.throws(() => nrSessionRequest({ ...active, nr_live_controls_version: 1 }, 42, { intensity: 0.5 }), /nr_toggle_unsupported/)
})

test('the reset switches off and puts every option back, the version 3 options only where the host has them', () => {
  assert.deepEqual(nrResetRequest(active, 42), { id: 42, enabled: false, scalePercent: 100, intensity: 1, style: 0, skinStructureStrength: 0, autoMask: false, uiCorrection: false, motionQuality: 0 })
  assert.deepEqual(nrResetRequest({ ...active, nr_live_controls_version: 2 }, 42), { id: 42, enabled: false, scalePercent: 100, intensity: 1, uiCorrection: false, motionQuality: 0 })
  assert.throws(() => nrResetRequest(active, 43), /nr_session_ended/)
})
