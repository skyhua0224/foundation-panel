// Turns the status the Panel reads from the host and the system into the rows
// of the status card. `say(path, params)` returns the sentence for a value key,
// so the rules stay testable without the i18n tables.

const WARN = new Set([
  'host_unreachable', 'host_unsupported', 'native_absent', 'transport_unsupported',
  'usbip_missing', 'usbip_not_ready',
])
const BAD = new Set(['usbip_version', 'usbip_unsigned'])

export function statusTone(state) {
  if (state === 'ready' || state === 'active') return 'ok'
  if (BAD.has(state)) return 'bad'
  if (WARN.has(state)) return 'warn'
  return 'neutral'
}

function nativeRow(host, say) {
  if (!host?.reachable || !host.tuning_available) return { tone: 'neutral', text: say('nativeUnknown') }
  return host.native_compiled
    ? { tone: 'ok', text: say('nativeBuilt') }
    : { tone: 'warn', text: say('nativeAbsent') }
}

function transportRow(transport, say) {
  if (!transport?.supported) return { tone: 'neutral', text: say('unavailable') }
  if (!transport.installed) return { tone: 'warn', text: say('notInstalled') }
  const version = transport.version
  if (!transport.version_supported) {
    return { tone: 'bad', text: say('mismatch', { version, pinned: transport.pinned_version }) }
  }
  return transport.ready
    ? { tone: 'ok', text: say('transportReady', { version }) }
    : { tone: 'warn', text: say('transportNotReady', { version }) }
}

function signatureRow(transport, say) {
  if (!transport?.installed || !transport.version_supported) return { tone: 'neutral', text: say('unavailable') }
  switch (transport.signature) {
    case 'signed':
      return {
        tone: 'ok',
        text: transport.signer ? say('signedBy', { signer: transport.signer }) : say('signed'),
      }
    case 'unsigned':
      return { tone: 'bad', text: say('unsigned') }
    case 'invalid':
      return { tone: 'bad', text: say('invalid') }
    case 'unknown':
      return { tone: 'neutral', text: say('unknown') }
    default:
      return { tone: 'neutral', text: say('notChecked') }
  }
}

function controllerRow(host, say) {
  if (!host?.tuning_available || !host.native_compiled) return { tone: 'neutral', text: say('unavailable') }
  const count = Number(host.active_controllers) || 0
  return count > 0 || host.native_active
    ? { tone: 'ok', text: say('controllerActive', { count: Math.max(count, 1) }) }
    : { tone: 'neutral', text: say('controllerIdle') }
}

export function statusRows(status, say) {
  if (!status) return []
  return [
    { id: 'native', ...nativeRow(status.host, say) },
    { id: 'transport', ...transportRow(status.transport, say) },
    { id: 'signature', ...signatureRow(status.transport, say) },
    { id: 'controller', ...controllerRow(status.host, say) },
  ]
}
