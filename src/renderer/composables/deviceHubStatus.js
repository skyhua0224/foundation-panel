/** Requires explicit shared cleanup for residual devices, unsupported versions or ABI conflicts. */
export function usbipTransportNeedsCleanup(status = {}) {
  return !status.externally_managed && Boolean(
    status.vhci_residual
    // error_code 是后端下发的结构化码（Rust status::detail_error_code），
    // 不得回退到解析 detail 文本。
    || (status.installed && (!status.version_valid || status.error_code === 'USBIP-EXEC-006')),
  )
}

export const MICROPHONE_BACKENDS = Object.freeze(['auto', 'vb_cable', 'usbip_experimental', 'disabled'])

/** The states of VB-CABLE the host reports (GET /api/audio/devices, vb_cable.state). */
export const VB_CABLE_STATES = Object.freeze(['ready', 'partial', 'disabled', 'unavailable', 'missing'])

/** VB-CABLE's state from the host's answer; `unknown` while there is none or for a state this version does not know. */
export function vbCableState(devices) {
  const state = devices?.vb_cable?.state
  return VB_CABLE_STATES.includes(state) ? state : 'unknown'
}

/** Not installed, or installed but not usable for another reason: the download is offered. */
export const vbCableNeedsInstall = (state) => VB_CABLE_STATES.includes(state) && state !== 'ready' && state !== 'disabled'

/** Installed, its device disabled in the Device Manager: the way out is to enable it there, not to download it again. */
export const vbCableNeedsEnable = (state) => state === 'disabled'

/** The job of the host that installs or enables VB-CABLE, as GET /api/audio/devices reports it (`vb_cable.job`). */
export function vbCableJob(devices) {
  const job = devices?.vb_cable?.job || {}
  const step = ['downloading', 'verifying', 'installing', 'enabling'].includes(job.step) ? job.step : ''
  return { running: job.running === true, step: job.running === true ? step || 'downloading' : step, failed: job.step === 'failed', errorCode: typeof job.error_code === 'string' ? job.error_code : '' }
}

/** What a failed job tells the person: the download or the pack, the signature, the missing administrator rights, or "still unavailable". */
export function vbCableErrorKind(code) {
  switch (code) {
    case 'VB_DOWNLOAD_FAILED': case 'VB_EXTRACT_FAILED': case 'VB_SETUP_MISSING': return 'download'
    case 'VB_SIGNATURE_INVALID': case 'VB_SIGNER_UNEXPECTED': return 'signature'
    case 'VB_NEEDS_ADMIN': return 'admin'
    case '': return ''
    default: return 'failed'
  }
}

/** The official page of VB-CABLE (the driver pack is downloaded from there). */
export const VB_CABLE_PAGE = 'https://vb-audio.com/Cable/'

/** Backends that need the usbip-win2 host component. */
const needsUsbip = (backend) => backend === 'usbip_experimental' || backend === 'auto'

export function microphoneStatusTone(status = {}) {
  const state = microphoneOverviewState(status)
  if (state === 'faulted') return 'state-error'
  if (['active', 'idle', 'capturing', 'streaming'].includes(state)) return 'state-ready'
  return ''
}

export function canTestMicrophone(status = {}, loading = false) {
  return Boolean(
    !loading
    && status.state !== 'unsupported'
    && MICROPHONE_BACKENDS.includes(status.configured_backend)
    && status.configured_backend !== 'disabled'
    // Only the USB microphone alone cannot fall back; it needs usbip-win2.
    && (status.configured_backend !== 'usbip_experimental' || status.component_available),
  )
}

/**
 * One state for the tab and the overview. The host's own states (absent, creating,
 * enumerating, idle, host_capturing, remote_active, destroying, device_faulted) only
 * describe the USB microphone; VB-CABLE reports `ready`.
 */
export function microphoneOverviewState(status = {}) {
  if (status.state === 'unsupported') return 'unsupported'
  if (!status.configured_backend) return 'unknown'
  if (status.configured_backend === 'disabled') return 'disabled'
  // USB endpoint fields do not describe VB-CABLE's capture state.
  if (status.active_backend === 'vb_cable') return 'active'
  if (status.state === 'device_faulted') return 'faulted'
  if (microphoneUsesUsbip(status) && status.active_backend) {
    switch (status.state) {
      case 'creating':
      case 'enumerating':
        return 'starting'
      case 'destroying':
        return 'stopping'
      case 'remote_active':
        return 'streaming'
      case 'host_capturing':
        return 'capturing'
      default:
        return 'idle'
    }
  }
  if (status.error_code && !status.active_backend) {
    return status.error_code === 'MIC_USBIP_COMPONENT_UNAVAILABLE' ? 'missing' : 'faulted'
  }
  if (status.configured_backend === 'usbip_experimental' && !status.component_available) return 'missing'
  return 'waiting'
}

/** usbip-win2 is missing although the chosen backend would use it. */
export function microphoneComponentMissing(status = {}) {
  return Boolean(
    status.state !== 'unsupported'
    && needsUsbip(status.configured_backend)
    && !status.component_available,
  )
}

/** The USB microphone buffer in milliseconds: 48 kHz, 16 bit, mono is 96 bytes per ms. */
export function microphoneBufferedMs(status = {}) {
  return Math.round(Number(status.buffered_bytes || 0) / 96)
}

export function microphoneUsesUsbip(status = {}) {
  return status.active_backend
    ? status.active_backend === 'usbip_experimental'
    : status.configured_backend === 'usbip_experimental'
}

/** Peak of the client microphone input since the previous status read, clamped to 0..1. */
export function microphoneInputLevel(status = {}) {
  const peak = Number(status.input_peak)
  return Number.isFinite(peak) ? Math.min(1, Math.max(0, peak)) : 0
}

/** True when the host saw a client microphone frame recently enough to call the route live. */
export function microphoneInputFresh(status = {}) {
  const ms = Number(status.input_active_ms)
  return Number.isFinite(ms) && ms >= 0 && ms < 3000
}

/** The host reported the live route instead of playing a test tone (a stream owns the microphone). */
export function microphoneTestIsLive(answer = {}) {
  return answer?.mode === 'live'
}
