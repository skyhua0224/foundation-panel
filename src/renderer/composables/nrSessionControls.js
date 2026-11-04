import { nrOverlayState } from './nrOverlayState.js'

// The defaults the overlay's 恢复 NR 默认值 sends (NrOverlayTool resetDefaults): off, full scale, full strength, every option at rest.
export const NR_DEFAULTS = Object.freeze({ scalePercent: 100, intensity: 1, style: 0, skinStructureStrength: 0, autoMask: false, uiCorrection: false, motionQuality: 0 })

const unit = (v) => typeof v === 'number' && Number.isFinite(v) && v >= 0 && v <= 1
const whole = (v, max) => Number.isInteger(v) && v >= 0 && v <= max
// option -> [controls version it needs, check of the value]; nr_live_set_enabled checks the same ranges on the host side
const OPTIONS = {
  intensity: [2, unit],
  uiCorrection: [2, (v) => typeof v === 'boolean'],
  motionQuality: [2, (v) => whole(v, 3)],
  style: [3, (v) => whole(v, 4)],
  skinStructureStrength: [3, unit],
  autoMask: [3, (v) => typeof v === 'boolean'],
}

// Validate the target again when committing an edit: a newly selected stream
// must never receive a value drafted for the previous one.
export function nrSessionRequest(pipeline, targetId, patch, online = true, { reset = false } = {}) {
  if (!pipeline || pipeline.id !== targetId) throw new Error('nr_session_ended')
  if (!online || !pipeline.nr_toggle_supported)
    throw new Error('nr_toggle_unsupported')
  if (
    ['warming_up', 'stopping', 'scaling'].includes(
      nrOverlayState(pipeline, online)
    )
  ) {
    throw new Error('nr_settings_pending')
  }
  const enabled = patch.enabled ?? pipeline.nr_requested_enabled
  const request = { id: targetId, enabled }
  const version = pipeline.nr_live_controls_version ?? 0
  if (patch.scalePercent !== undefined) {
    // the scale is set on a running enhancement; the reset sends it together with switching off, as the overlay does
    if (!(version >= 2) || (!enabled && !reset))
      throw new Error('nr_toggle_unsupported')
    if (
      !Number.isInteger(patch.scalePercent) ||
      patch.scalePercent < 20 ||
      patch.scalePercent > 100 ||
      patch.scalePercent % 5
    ) {
      throw new Error('nr_request_invalid')
    }
    request.scalePercent = patch.scalePercent
  }
  for (const [key, [needs, valid]] of Object.entries(OPTIONS)) {
    if (patch[key] === undefined) continue
    if (!(version >= needs)) throw new Error('nr_toggle_unsupported')
    if (!valid(patch[key])) throw new Error('nr_request_invalid')
    request[key] = patch[key]
  }
  return request
}

/** The request of 恢复 NR 默认值: switch off and put every option back (the options of controls version 3 only where the host has them). */
export function nrResetRequest(pipeline, targetId, online = true) {
  const version = pipeline?.nr_live_controls_version ?? 0
  const patch = { enabled: false, ...NR_DEFAULTS }
  if (!(version >= 3)) { delete patch.style; delete patch.skinStructureStrength; delete patch.autoMask }
  return nrSessionRequest(pipeline, targetId, patch, online, { reset: true })
}
