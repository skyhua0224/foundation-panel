import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { nrSessionRequest, nrResetRequest, focusLeaves, startsSliderEdit } from './nrSessionControls.js'
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

test('the reset preserves the global switch and puts every option back, the version 3 options only where the host has them', () => {
  assert.deepEqual(nrResetRequest(active, 42), { id: 42, enabled: true, scalePercent: 100, intensity: 1, style: 0, skinStructureStrength: 0, autoMask: false, uiCorrection: false, motionQuality: 0 })
  assert.deepEqual(nrResetRequest({ ...active, nr_live_controls_version: 2 }, 42), { id: 42, enabled: true, scalePercent: 100, intensity: 1, uiCorrection: false, motionQuality: 0 })
  assert.throws(() => nrResetRequest(active, 43), /nr_session_ended/)
})

test('the focus going from one control of the live controls to another does not drop the edit, leaving them does', () => {
  const inside = { id: 'handle' }
  const section = { contains: (node) => node === inside }
  assert.equal(focusLeaves(section, inside), false, 'a press on the track of the next slider')
  assert.equal(focusLeaves(section, null), true, 'to nothing (the page, another window)')
  assert.equal(focusLeaves(section, { id: 'outside' }), true)
  assert.equal(focusLeaves(null, inside), true)
  const view = readFileSync(new URL('../components/NrSessionControls.vue', import.meta.url), 'utf8')
  assert.match(view, /@focusout="\(event\) => \{ if \(focusLeaves\(event\.currentTarget, event\.relatedTarget\)\) cancelEdit\(\) \}"/)
  assert.ok(!/@focusout="cancelEdit"/.test(view), 'a focus change inside the section must not cancel the edit')
})

test('only a press on a slider or a value key on its handle begins an edit: Tab and the other controls do not', () => {
  const inside = (selectors) => ({ closest: (selector) => (selectors.includes(selector) ? {} : null) })
  const slider = inside(['.m-slider', '[role=slider]'])
  const seg = inside([])
  assert.equal(startsSliderEdit({ type: 'pointerdown', target: slider }), true)
  assert.equal(startsSliderEdit({ type: 'pointerdown', target: seg }), false, 'a press on a switch or a segmented control')
  assert.equal(startsSliderEdit({ type: 'keydown', key: 'ArrowRight', target: slider }), true)
  assert.equal(startsSliderEdit({ type: 'keydown', key: 'Home', target: slider }), true)
  assert.equal(startsSliderEdit({ type: 'keydown', key: 'Tab', target: slider }), false, 'Tab moves the focus, it edits nothing')
  assert.equal(startsSliderEdit({ type: 'keydown', key: 'ArrowRight', target: seg }), false, 'the arrows of a segmented control')
  assert.equal(startsSliderEdit({ type: 'keydown', key: 'ArrowRight', target: {} }), false)
  assert.equal(startsSliderEdit(null), false)
  const view = readFileSync(new URL('../components/NrSessionControls.vue', import.meta.url), 'utf8')
  assert.match(view, /@pointerdown\.capture="pressEdit" @keydown\.capture="pressEdit"/)
  assert.ok(!/@keydown\.capture="beginEdit"/.test(view))
})
