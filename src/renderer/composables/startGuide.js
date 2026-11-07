// `--start-guide`: open the first-run guide at its first step, whatever progress was saved.
// The host keeps one slot (src-tauri/src/start_guide.rs). This module remembers that the next
// guide mount should take it, so a window that is already on the guide can be sent back to the start.

let armed = false

/** The next guide mount starts from the first step. A second arm before that mount is the same request. */
export function armStartGuide() {
  armed = true
}

/** True once, for the mount that should ignore saved progress. */
export function consumeStartGuide() {
  const value = armed
  armed = false
  return value
}

/**
 * The guide to show. `fromStart` is `--start-guide`: the whole first-run flow, not the upgrade flow.
 * Otherwise `entry` is the decision the panel already made.
 */
export function guideOnLaunch({ fromStart = false, entry = null } = {}) {
  if (fromStart) return { mode: 'fresh', skip: [], fromStart: true }
  return entry
}

/**
 * Take the request the host left (take_pending_start_guide) and open the guide. Called on the
 * `start-guide-pending` ping and once at mount: the host empties its slot on take, so it opens once.
 */
export async function deliverPendingStartGuide(invoke, open) {
  let requested = false
  try { requested = await invoke('take_pending_start_guide') } catch { return false }
  if (requested !== true) return false
  open()
  return true
}
