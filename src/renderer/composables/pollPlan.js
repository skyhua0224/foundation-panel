// How often the home overview asks the host who is streaming. The host has no way to push (the audit api-exposure-20261107 found no events route: every /api/*events answers 444), so the poll is
// shortened where a change is likely: for a few seconds after the panel is looked at again (it comes back to the front, or its window is shown) and while the host reports a session that is not
// RUNNING yet (a stream that is starting). One request of /api/runtime/sessions per second in those windows, three seconds otherwise.
export const POLL_BASE_MS = 3000
export const POLL_BOOST_MS = 1000
export const BOOST_WINDOW_MS = Object.freeze({ returned: 15000, starting: 10000 })

/** The delay before the next poll. */
export const pollDelay = (now, boostUntil) => (now < boostUntil ? POLL_BOOST_MS : POLL_BASE_MS)

/** The end of the fast window after `reason`, never shorter than the one that is running. */
export const boostUntilFor = (now, reason, current = 0) => Math.max(current, now + (BOOST_WINDOW_MS[reason] || 0))

/** True when the host lists a session that is not running yet (CONNECTING, STARTING, …). */
export const hasStartingSession = (sessions) => (Array.isArray(sessions) ? sessions : []).some((s) => s && s.state && String(s.state).toUpperCase() !== 'RUNNING')
