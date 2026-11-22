// What the page shows about the clipboard sync agent (the Rust command `clipboard_sync_status`). One pure function: the status in, the chip and the rows out, so the sidebar chip, the diagnostics page and the
// diagnostics bundle all say the same thing. The failure class is one of the fixed words of the agent (clipboard_health.rs); a word the page does not know is shown as "other", never as the raw text.

const KNOWN_ERRORS = ['connect_refused', 'timeout', 'tls', 'http_401', 'http_403', 'http_5xx', 'stream_ended', 'credential_missing', 'heartbeat', 'other']

/** The state of the sync as one of: off | disabled | unknown | connecting | degraded | disconnected | connected. */
export function clipboardSyncState(status) {
  if (!status) return 'unknown'
  if (!status.agent_active || status.transport_state === 'stopped') return 'off'
  if (status.service_allowed === false) return 'disabled'
  if (status.transport_state === 'degraded') return 'degraded'
  if (status.transport_state === 'connecting') return 'connecting'
  if (status.transport_state === 'disconnected') return 'disconnected'
  if (status.transport_state === 'connected') {
    // the host's setting could not be read: connected on this side, but nobody has confirmed the host accepts it
    return status.service_allowed === true ? 'connected' : 'unknown'
  }
  return 'unknown'
}

/** True only when the agent is connected, the host heard it, and the host's own setting says on. */
export const clipboardSyncWorks = (status) => clipboardSyncState(status) === 'connected'

const TONES = { connected: 'ok', degraded: 'warn', disconnected: 'warn', connecting: 'info', disabled: 'neutral', off: 'neutral', unknown: 'neutral' }

const errorWord = (word) => (KNOWN_ERRORS.includes(word) ? word : 'other')

function agoText(ms, now, texts) {
  if (!ms) return texts.never
  const seconds = Math.max(0, Math.round((now - ms) / 1000))
  if (seconds < 90) return texts.secondsAgo.replace('{n}', String(seconds))
  return texts.minutesAgo.replace('{n}', String(Math.round(seconds / 60)))
}

/**
 * @param {object|null} status the answer of `clipboard_sync_status`
 * @param {object} texts the `clipboardSync.diag` block of the locale
 * @returns {{ state: string, tone: string, label: string, rows: {id: string, title: string, text: string}[] }}
 */
export function clipboardStatusView(status, texts, now = Date.now()) {
  const state = clipboardSyncState(status)
  const rows = []
  if (status) {
    rows.push({ id: 'agent', title: texts.agent, text: status.agent_active ? texts.running : texts.notRunning })
    rows.push({ id: 'setting', title: texts.setting, text: status.service_allowed === true ? texts.settingOn : status.service_allowed === false ? texts.settingOff : texts.settingUnknown })
    rows.push({ id: 'host', title: texts.hostAgent, text: status.host_gui_alive === true ? texts.hostSees : status.host_gui_alive === false ? texts.hostDoesNotSee : texts.hostUnknown })
    rows.push({ id: 'heartbeat', title: texts.heartbeat, text: status.last_heartbeat_ok_ms ? agoText(status.last_heartbeat_ok_ms, now, texts) : texts.never })
    if (status.heartbeat_error && status.heartbeat_error !== 'none') rows.push({ id: 'heartbeatError', title: texts.heartbeatError, text: texts.errors[errorWord(status.heartbeat_error)] })
    if (status.last_error_class && status.last_error_class !== 'none') rows.push({ id: 'lastError', title: texts.lastError, text: texts.errors[errorWord(status.last_error_class)] })
    if (status.capability_error && status.capability_error !== 'none') rows.push({ id: 'capabilityError', title: texts.capabilityError, text: texts.errors[errorWord(status.capability_error)] })
    if (status.last_connected_at_ms) rows.push({ id: 'connected', title: texts.lastConnected, text: agoText(status.last_connected_at_ms, now, texts) })
  }
  return { state, tone: TONES[state], label: texts.states[state], rows }
}

/** The part of the status the diagnostics bundle carries: fixed words, counts and times. The text of the last error (it can hold an address) stays out. */
export function clipboardBundleFacts(status) {
  if (!status) return null
  const { agent_active, service_allowed, transport_state, last_connected_at_ms, last_error_class, host_gui_alive, host_session_count, last_heartbeat_ok_ms, heartbeat_error, capability_error } = status
  return { agent_active, service_allowed, transport_state, last_connected_at_ms, last_error_class, host_gui_alive, host_session_count, last_heartbeat_ok_ms, heartbeat_error, capability_error, state: clipboardSyncState(status) }
}
