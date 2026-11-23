import { computed, onActivated, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue'
import { ElMessage, ElMessageBox } from './notifyMessage.js'
import { dualsense, sunshine, usbip } from '../tauri-adapter.js'
import compatibility from '../../../src-tauri/component-compatibility.json' with { type: 'json' }
import { useI18n } from '../desktop/i18n/index.js'
import { useDeviceComponentOperation } from './useDeviceComponentOperation.js'
import { usbipStatusChannel } from './deviceStatusStore.js'
import { usbipCanReplaceLegacy, usbipResidualIsStale, usbipTransportNeedsCleanup } from './deviceHubStatus.js'
import { clearFixResume, describeFixResult, fixConfirmText, fixGuideText, fixProgressText, loadFixResume, runUsbipFix, saveFixResume, usbipFixOffered, usbipFixOfferedByStatus, usbipFixRawText } from './usbipFix.js'

// what the last one-click fix answered, as raw text: shared by every panel that shows the transport, kept until the next fix
const fixRaw = ref('')
const FIX_MESSAGE_ID = 'usbip-fix'

/** Extracts a fixed-vocabulary telemetry error code (never user text or paths). */
export function usbipErrorReportCode(error) {
  const message = String(error || '')
  const codes = message.match(/USBIP-[A-Z]+-\d{3}/g) || []
  const primary = (codes[0] === 'USBIP-SETUP-007' && codes[1]) ? codes[1] : (codes[0] || '')
  if (!primary) return 'usbip-unknown'
  const hex = message.match(/\b0x[0-9a-f]+\b/i)?.[0]?.toLowerCase()
  const exit = message.match(/\bexit code (-?\d+)\b/i)?.[1]
  const sub = hex || (exit ? `exit-${exit}` : '')
  const raw = sub ? `${primary}.${sub}` : primary
  return raw.toLowerCase().replace(/[^a-z0-9._-]+/g, '-').slice(0, 40).replace(/-+$/, '')
}

/** Dispatches a structured USB/IP error event without any user text. */
export function emitUsbipErrorReport(error, reporter) {
  const code = usbipErrorReportCode(error)
  if (typeof reporter === 'function') {
    reporter({ component: 'usbip', code, severity: 'error' })
    return code
  }
  if (typeof globalThis.window?.dispatchEvent === 'function' && typeof globalThis.CustomEvent === 'function') {
    globalThis.window.dispatchEvent(new globalThis.CustomEvent('foundation-error-event', {
      detail: { component: 'usbip', code, severity: 'error' },
    }))
  }
  return code
}

/** Resolves the most specific translated user message for a structured USB/IP error. */
export function formatUsbipFriendlyError(error, usbTexts = {}, componentTexts = {}) {
  const message = String(error || '')
  const codes = message.match(/USBIP-[A-Z]+-\d{3}/g) || []
  const code = (codes[0] === 'USBIP-SETUP-007' && codes[1]) ? codes[1] : codes[0]
  if (code === 'USBIP-SETUP-018' && componentTexts.operationBusy) return componentTexts.operationBusy
  const translated = code && usbTexts.errors?.[code]
  return translated || usbTexts.unknownError || 'The USB/IP operation did not complete.'
}

/** Coordinates shared transport maintenance and refreshes actual state after failed operations. */
export function useUsbTransport(options = {}) {
  const { t } = useI18n()
  const sharedOperation = useDeviceComponentOperation()
  const status = reactive({
    supported: true,
    installed: false,
    externally_managed: false,
    replaceable: false,
    ready: false,
    version: '',
    version_valid: false,
    reboot_recommended: false,
    vhci_residual: false,
    residual_kind: '',
    cleanup_pending_restart: false,
    cleanup_resume: false,
    cleanup_resume_op: '',
    last_cleanup_error: '',
    last_install_error: '',
    install_pending_restart: false,
    install_resume: false,
    error_code: '',
    driver_signature: '',
    attached_devices: [],
    detail: '',
  })
  const statusLoading = ref(false)
  const statusLoaded = ref(false)
  const statusProbeFailed = ref(false)
  const installing = ref(false)
  const cleaning = ref(false)
  const replacing = ref(false)
  const exporting = ref(false)
  const fixing = ref(false)
  // The cleanup that waited for a restart is continued once per run of the panel, so a node that never goes cannot loop.
  let resumeStarted = false
  let resumeAnnounced = false
  let resumeRetryTimer
  let resumeBusyRetries = 0
  let disposed = false
  // A repair that waited for a restart continues once Windows has started after it was kept. When this Windows session began comes from the
  // native side (the clock of the cleanup's restart marker), read once; until it is known nothing is claimed.
  let bootMs = 0
  const readBoot = async () => {
    try {
      const secs = Number((await usbip.bootSecs())?.data)
      bootMs = Number.isFinite(secs) && secs > 0 ? secs * 1000 : 0
    } catch { bootMs = 0 }
  }
  const hasFixResume = () => Boolean(loadFixResume(globalThis.localStorage, { bootMs }))
  let statusRefreshPromise = null

  function retryBusyResume(error, auto) {
    if (!auto || !String(error || '').includes('USBIP-SETUP-018') || resumeBusyRetries >= 36) return false
    if (!resumeRetryTimer && !disposed) {
      resumeBusyRetries += 1
      // The native preflight checks the mutex before UAC, so retrying a busy
      // automatic recovery does not repeatedly ask for administrator access.
      resumeRetryTimer = setTimeout(() => {
        resumeRetryTimer = undefined
        resumeStarted = false
        if (!disposed) void refreshStatus({ force: true })
      }, 5000)
    }
    return true
  }
  onBeforeUnmount(() => { disposed = true; clearTimeout(resumeRetryTimer) })

  const attachedDevices = computed(() => status.attached_devices || [])
  const needsCleanup = computed(() => usbipTransportNeedsCleanup(status))
  const canReplaceLegacy = computed(() => usbipCanReplaceLegacy(status))
  const staleResidual = computed(() => usbipResidualIsStale(status))
  // the operations of this window that change the transport or read it for a change; the fix runs them itself, so they do not wait for it
  const operationBlocked = () => Boolean(sharedOperation.busyElsewhere.value || installing.value || cleaning.value || replacing.value || exporting.value)
  const transportBusy = computed(() => Boolean(operationBlocked() || fixing.value))
  // the repair is offered for every state it knows (usbipFixInventory.js); for an installation of the person that the host will not replace it
  // offers the steps to remove it by hand, and changes nothing
  const fixOffered = computed(() => statusLoaded.value && !statusProbeFailed.value && usbipFixOfferedByStatus(status))
  const statusClass = computed(() => {
    if (status.ready) return 'state-ready'
    if (!statusLoaded.value) return ''
    if (statusProbeFailed.value) return 'state-error'
    if (!status.supported) return ''
    return status.installed ? 'state-error' : 'state-update_available'
  })
  const statusTitle = computed(() => {
    if (!statusLoaded.value) return t.value.deviceHub.usb.checkingTransport
    if (statusProbeFailed.value) return t.value.deviceHub.usb.statusUnavailable
    if (!status.supported) return t.value.deviceHub.usb.transportUnsupported
    if (status.externally_managed) return t.value.deviceHub.usb.externallyManaged
    if (status.ready) return t.value.deviceHub.usb.transportReady
    if (!status.installed) return t.value.deviceHub.usb.transportMissing
    return t.value.deviceHub.usb.transportNeedsRepair
  })

  function applyStatus(next) {
    if (next) Object.assign(status, next)
  }

  function friendlyError(error) {
    return formatUsbipFriendlyError(error, t.value.deviceHub.usb, t.value.deviceHub.vbComponent)
  }

  function reportError(error) {
    emitUsbipErrorReport(error, options?.onErrorEvent)
  }

  /** The text of a cleanup failure: what Windows said and the next step; the codes themselves are in the diagnostics. */
  function cleanupError(error) {
    const text = friendlyError(error)
    const code = String(error || '').match(/^(USBIP-CLEAN-\d{3})/)?.[1]
    if (!code) return text
    const usb = t.value.deviceHub.usb
    // 006 to 008 already say what to do; the others get the generic next step and, for a refusal, the manual way.
    if (['USBIP-CLEAN-006', 'USBIP-CLEAN-008'].includes(code)) return text
    return `${text} ${code === 'USBIP-CLEAN-007' ? usb.manualRemoval : usb.cleanupFailedHint}`
  }

  function applyProbeFailure(detail) {
    statusProbeFailed.value = true
    status.ready = false
    status.attached_devices = []
    status.vhci_residual = false
    status.detail = detail
  }

  /** Resolves how the read went: 'ok', 'failed' (the host did not answer; the page shows it as unavailable) or 'disposed' (nothing was applied). */
  async function performStatusRefresh({ force = false } = {}) {
    statusLoading.value = true
    let outcome = 'ok'
    try {
      const result = await usbipStatusChannel.fetch({ force })
      if (disposed) return 'disposed'
      if (result?.success) {
        statusProbeFailed.value = false
        applyStatus(result.data)
        // a driver Windows would refuse reads ready here: its signature is part of the state the repair is offered for
        let signature = ''
        if (result.data?.installed && result.data?.version_valid) {
          try { signature = (await usbip.driverSignature())?.data || '' } catch { signature = '' }
        }
        if (disposed) return 'disposed'
        status.driver_signature = String(signature)
      } else {
        outcome = 'failed'
        applyProbeFailure(result?.message || '')
      }
    } catch (error) {
      if (disposed) return 'disposed'
      outcome = 'failed'
      applyProbeFailure(String(error || ''))
    } finally {
      statusLoaded.value = true
      statusLoading.value = false
    }
    if (disposed) return 'disposed'
    // Windows restarted since a cleanup waited for it and the leftover is still there: continue the same operation (one prompt for
    // administrator rights). The one chance is spent only when the operation can start now; while another one holds the transport
    // the next refresh asks again.
    const resumeStale = status.cleanup_resume_op === 'stale'
    // a repair that waited for a restart (fix.resume) continues the cleanup itself, with the rest of its steps
    if (status.cleanup_resume && !resumeStarted && !hasFixResume() && !statusProbeFailed.value && !transportBusy.value && (resumeStale || !status.externally_managed)) {
      resumeStarted = true
      if (!resumeAnnounced) ElMessage.info(t.value.deviceHub.usb.resumeCleanup)
      resumeAnnounced = true
      queueMicrotask(() => {
        if (!disposed) void (resumeStale ? removeStale({ auto: true }) : cleanupResidual({ auto: true }))
      })
    } else if (status.install_resume && !resumeStarted && !hasFixResume() && !statusProbeFailed.value && !transportBusy.value && !status.externally_managed && !status.vhci_residual) {
      resumeStarted = true
      if (!resumeAnnounced) ElMessage.info(t.value.deviceHub.usb.resumeInstall)
      resumeAnnounced = true
      queueMicrotask(() => { if (!disposed) void installTransport({ auto: true }) })
    }
    return outcome
  }

  /** Resolves 'ok', 'failed' or 'disposed': how the read that answers this call went (the one already running, when this call joins it). */
  async function refreshStatus(options = {}) {
    if (disposed) return 'disposed'
    const afterCurrent = options?.afterCurrent === true
    const force = options?.force === true
    const activeRefresh = statusRefreshPromise
    if (activeRefresh) {
      const joined = await activeRefresh
      if (disposed) return 'disposed'
      if (!afterCurrent) return joined
    }

    const nextRefresh = performStatusRefresh({ force })
    statusRefreshPromise = nextRefresh
    try {
      return await nextRefresh
    } finally {
      if (statusRefreshPromise === nextRefresh) statusRefreshPromise = null
    }
  }

  // The three steps below answer { ok, data | error }, or { ok: false, skipped: true } when they did not start. `quiet` keeps their own messages
  // away (the fix has its own), `confirmed` means the person already answered the question.
  const installRaw = async (installOptions = {}) => {
    if (operationBlocked() || status.externally_managed) return { ok: false, skipped: true }
    const auto = installOptions?.auto === true
    const quiet = installOptions?.quiet === true
    if (auto && disposed) return { ok: false, skipped: true }
    installing.value = true
    if (!auto && installOptions?.confirmed !== true) {
      try {
        await ElMessageBox.confirm(
          t.value.deviceHub.usb.installConfirm,
          t.value.deviceHub.usb.installTransport,
          { confirmButtonText: t.value.deviceHub.usb.continue, cancelButtonText: t.value.deviceHub.usb.cancel, type: 'warning' },
        )
      } catch { installing.value = false; return { ok: false, skipped: true } }
    }
    // Do not let an in-flight status refresh apply stale data over the result
    // of the operation that is about to run.
    if (statusRefreshPromise) await statusRefreshPromise
    if (auto && disposed) { installing.value = false; return { ok: false, skipped: true } }
    sharedOperation.markChanged()
    usbipStatusChannel.invalidate()
    let result
    try {
      result = await usbip.installTransport()
    } catch (error) {
      if (retryBusyResume(error, auto)) return { ok: false, skipped: true }
      reportError(error)
      await refreshStatus({ force: true })
      if (!status.last_install_error && error) status.last_install_error = String(error)
      if (!quiet) ElMessage.error(friendlyError(error))
      return { ok: false, error: String(error), reported: true }
    } finally {
      installing.value = false
    }
    if (!result?.success) {
      if (retryBusyResume(result?.message, auto)) return { ok: false, skipped: true }
      reportError(result?.message)
      await refreshStatus({ force: true })
      if (!status.last_install_error && result?.message) status.last_install_error = String(result.message)
      if (!quiet) ElMessage.error(friendlyError(result?.message))
      return { ok: false, error: String(result?.message), reported: true }
    }
    applyStatus(result.data)
    usbipStatusChannel.prime(result)
    if (!quiet) {
      if (result.data?.ready) ElMessage.success(t.value.deviceHub.usb.installSuccess)
      else ElMessage.warning(t.value.deviceHub.usb.rebootRequired)
    }
    return { ok: true, data: result.data }
  }
  const installTransport = sharedOperation.wrap(installRaw)

  const cleanupRaw = async (options = {}) => {
    if (operationBlocked() || status.externally_managed) return { ok: false, skipped: true }
    const removing = !needsCleanup.value
    const auto = options?.auto === true
    const quiet = options?.quiet === true
    if (auto && disposed) return { ok: false, skipped: true }
    cleaning.value = true
    // The person confirmed this cleanup before the restart (or in the fix); continuing it asks nothing new.
    if (!auto && options?.confirmed !== true) {
      try {
        await ElMessageBox.confirm(
          t.value.deviceHub.usb.cleanupConfirm,
          removing ? t.value.deviceHub.components.uninstallTransport : t.value.deviceHub.usb.cleanupResidual,
          { confirmButtonText: removing ? t.value.systemTools.uninstall : t.value.deviceHub.usb.continue, cancelButtonText: t.value.deviceHub.usb.cancel, type: 'warning', danger: removing },
        )
      } catch { cleaning.value = false; return { ok: false, skipped: true } }
    }
    if (statusRefreshPromise) await statusRefreshPromise
    if (auto && disposed) { cleaning.value = false; return { ok: false, skipped: true } }
    sharedOperation.markChanged()
    usbipStatusChannel.invalidate()
    let result
    try {
      result = await usbip.cleanupTransport()
    } catch (error) {
      if (retryBusyResume(error, auto)) return { ok: false, skipped: true }
      reportError(error)
      await refreshStatus({ force: true })
      if (!quiet) ElMessage.error(cleanupError(error))
      return { ok: false, error: String(error), reported: true }
    } finally {
      cleaning.value = false
    }
    if (!result?.success) {
      if (retryBusyResume(result?.message, auto)) return { ok: false, skipped: true }
      reportError(result?.message)
      // The failed cleanup may still have removed some devnodes; refresh so the
      // residual flag reflects the actual state instead of the stale snapshot.
      await refreshStatus({ force: true })
      if (!quiet) ElMessage.error(cleanupError(result?.message))
      return { ok: false, error: String(result?.message), reported: true }
    }
    applyStatus(result.data)
    usbipStatusChannel.prime(result)
    if (!quiet) {
      if (result.data?.cleanup_pending_restart) ElMessage.warning(t.value.deviceHub.usb.restartPending)
      else if (result.data?.vhci_residual) ElMessage.error(cleanupError('USBIP-CLEAN-002'))
      else ElMessage.success(removing ? t.value.deviceHub.components.transportRemoved : t.value.deviceHub.usb.cleanupSuccess)
    }
    return { ok: true, data: result.data }
  }
  const cleanupResidual = sharedOperation.wrap(cleanupRaw)

  /** Removes the older controllers beside the working one; the live controller stays and nothing is reinstalled. */
  const removeStaleRaw = async (options = {}) => {
    if (operationBlocked() || !staleResidual.value) return { ok: false, skipped: true }
    const auto = options?.auto === true
    const quiet = options?.quiet === true
    if (auto && disposed) return { ok: false, skipped: true }
    cleaning.value = true
    if (statusRefreshPromise) await statusRefreshPromise
    if (auto && disposed) { cleaning.value = false; return { ok: false, skipped: true } }
    sharedOperation.markChanged()
    usbipStatusChannel.invalidate()
    let result
    try {
      result = await usbip.removeStaleNodes()
    } catch (error) {
      if (retryBusyResume(error, auto)) return { ok: false, skipped: true }
      reportError(error)
      await refreshStatus({ force: true })
      if (!quiet) ElMessage.error(cleanupError(error))
      return { ok: false, error: String(error), reported: true }
    } finally {
      cleaning.value = false
    }
    if (!result?.success) {
      if (retryBusyResume(result?.message, auto)) return { ok: false, skipped: true }
      reportError(result?.message)
      await refreshStatus({ force: true })
      if (!quiet) ElMessage.error(cleanupError(result?.message))
      return { ok: false, error: String(result?.message), reported: true }
    }
    applyStatus(result.data)
    usbipStatusChannel.prime(result)
    if (!quiet) {
      if (result.data?.cleanup_pending_restart) ElMessage.warning(t.value.deviceHub.usb.restartPending)
      else if (result.data?.vhci_residual) ElMessage.error(cleanupError('USBIP-CLEAN-002'))
      else ElMessage.success(t.value.deviceHub.usb.staleSuccess)
    }
    return { ok: true, data: result.data }
  }
  const removeStale = sharedOperation.wrap(removeStaleRaw)

  /** Writes the diagnostics file (device tree, services, driver packages, Windows version, last cleanup answer) and shows it. */
  async function exportDiagnostics() {
    if (exporting.value) return
    exporting.value = true
    try {
      const result = await usbip.exportDiagnostics()
      if (!result?.success) return ElMessage.error(friendlyError(result?.message))
      ElMessage.success(t.value.deviceHub.usb.diagnosticsSaved.replace('{path}', String(result.data || '')))
    } catch (error) {
      ElMessage.error(friendlyError(error))
    } finally {
      exporting.value = false
    }
  }

  /** The person's own request to replace an external full usbip-win2; the host decides again and rolls back on failure. */
  const replaceRaw = async (options = {}) => {
    if (operationBlocked() || !canReplaceLegacy.value || attachedDevices.value.length) return { ok: false, skipped: true }
    const quiet = options?.quiet === true
    replacing.value = true
    if (options?.confirmed !== true) {
      try {
        await ElMessageBox.confirm(
          t.value.deviceHub.usb.replaceConfirm,
          t.value.deviceHub.usb.replaceLegacy,
          { confirmButtonText: t.value.deviceHub.usb.replaceLegacy, cancelButtonText: t.value.deviceHub.usb.cancel, type: 'warning', danger: true },
        )
      } catch { replacing.value = false; return { ok: false, skipped: true } }
    }
    if (statusRefreshPromise) await statusRefreshPromise
    sharedOperation.markChanged()
    usbipStatusChannel.invalidate()
    let result
    try {
      result = await usbip.replaceLegacyTransport()
    } catch (error) {
      reportError(error)
      await refreshStatus({ force: true })
      if (!status.last_install_error && error) status.last_install_error = String(error)
      if (!quiet) ElMessage.error(friendlyError(error))
      return { ok: false, error: String(error), reported: true }
    } finally {
      replacing.value = false
    }
    if (!result?.success) {
      reportError(result?.message)
      // A failed replacement may have changed the system (rolled back to "not installed"): show what is really there.
      await refreshStatus({ force: true })
      if (!status.last_install_error && result?.message) status.last_install_error = String(result.message)
      if (!quiet) ElMessage.error(friendlyError(result?.message))
      return { ok: false, error: String(result?.message), reported: true }
    }
    applyStatus(result.data)
    usbipStatusChannel.prime(result)
    if (!quiet) {
      if (result.data?.ready) ElMessage.success(t.value.deviceHub.usb.replaceSuccess)
      else ElMessage.warning(t.value.deviceHub.usb.rebootRequired)
    }
    return { ok: true, data: result.data }
  }
  const replaceLegacy = sharedOperation.wrap(replaceRaw)

  /** An error toast of the panel; the version mismatch carries the fix on the toast itself. */
  function notifyError(error) {
    const message = friendlyError(error)
    if (!usbipFixOffered(error)) return ElMessage.error(message)
    return ElMessage.error({ message, actionText: t.value.deviceHub.usb.fixAction, duration: 15000, onClick: () => { void fixTransport() } })
  }

  /** Windows restarts 30 seconds after the person said so; `shutdown /a` still cancels it. */
  async function restartWindows() {
    const usb = t.value.deviceHub.usb
    try {
      await ElMessageBox.confirm(usb.fixRestartConfirm, usb.fixRestartNow, { confirmButtonText: usb.fixRestartNow, cancelButtonText: usb.cancel, type: 'warning' })
    } catch { return }
    const result = await usbip.restartWindows()
    if (!result?.success) ElMessage.error(friendlyError(result?.message))
  }

  /** The raw answer of every step of the last fix, and what the transport says about itself. */
  function showFixDiagnostics() {
    return ElMessageBox.alert(fixRaw.value || status.detail || '', t.value.deviceHub.usb.diagnosticDetails)
  }

  /**
   * The one-click fix of a tool or driver that differs from the pinned version: reads the status, asks once, ends the stream through the
   * host, then runs the cleanup and install (or the confirmed replacement) of this page, and says how it went in the message at the bottom.
   */
  /** The numbered steps for a failure that software cannot finish; the raw diagnostics are one touch away in the same dialog. */
  async function showFixGuide(id) {
    const usb = t.value.deviceHub.usb
    try {
      await ElMessageBox.confirm(fixGuideText(id, usb, compatibility.usbip.version), usb.fixTitle, { confirmButtonText: usb.diagnosticDetails, cancelButtonText: usb.cancel, type: 'info' })
    } catch { return }
    await showFixDiagnostics()
  }

  /** What the person reads after the steps: the status as the page shows it, and whether the DualSense native channel agrees. */
  async function verifyRepair() {
    const read = await refreshStatus({ force: true, afterCurrent: true })
    if (read !== 'ok') return { ok: false, error: 'USBIP-VERIFY-001: the transport status could not be read after the repair' }
    if (!status.ready || status.vhci_residual || status.error_code) {
      return { ok: false, error: `USBIP-VERIFY-001: USB/IP is not ready after the repair${status.detail ? `: ${status.detail}` : ''}` }
    }
    let state = ''
    try { state = (await dualsense.getStatus())?.data?.state || '' } catch { state = '' }
    if (['usbip_missing', 'usbip_version', 'usbip_unsigned', 'usbip_not_ready'].includes(state)) {
      return { ok: false, error: `USBIP-VERIFY-002: the DualSense native channel reports ${state}` }
    }
    // a host that does not answer cannot say: the transport is checked, the native channel is not
    return { ok: true, note: state === 'host_unreachable' || !state ? 'host-unreachable' : '', raw: `dualsense: ${state || 'unknown'}` }
  }

  /**
   * The one-click repair of the USB/IP transport (usbipFixInventory.js lists every state): reads the status, asks once, ends the stream through
   * the host, then runs the steps of this page that the state needs, checks the result, and says how it went in the message at the bottom.
   * `options.resume` continues what was confirmed before a restart.
   */
  const fixTransport = sharedOperation.wrap(async (options = {}) => {
    if (operationBlocked() || fixing.value) return
    fixing.value = true
    const resume = options?.resume || null
    const usb = () => t.value.deviceHub.usb
    const version = compatibility.usbip.version
    let message = null
    const show = (tone, opts) => { message = ElMessage[tone]({ id: FIX_MESSAGE_ID, ...opts }) }
    const progress = (state) => {
      if (state.phase === 'checking') show('info', { message: resume ? usb().fixResumeStart : `${usb().fixStepCheck}…`, duration: 0 })
      else if (state.phase === 'running') show('info', { message: fixProgressText(state, usb(), version), duration: 0 })
    }
    let finished
    try {
      sharedOperation.markChanged()
      finished = await runUsbipFix({
        refresh: async () => {
          usbipStatusChannel.invalidate()
          // the repair starts only from a status that was just read: a failed or abandoned read is the reason it does not
          const read = await refreshStatus({ force: true, afterCurrent: true })
          if (read === 'disposed') throw new Error('USB/IP status refresh was abandoned: the page was closed')
          if (read !== 'ok') throw new Error(`USB/IP status refresh failed${status.detail ? `: ${status.detail}` : ''}`)
          if (disposed) throw new Error('USB/IP status refresh was abandoned: the page was closed')
          return { ...status, attached_devices: [...(status.attached_devices || [])] }
        },
        sessions: async () => {
          // a page that was closed cannot say what runs: unknown stops the repair before anything changes
          if (disposed) return null
          try {
            const sessions = await sunshine.askActiveSessions()
            return Array.isArray(sessions) ? sessions : null
          } catch { return null }
        },
        confirm: ({ plan }) => (disposed ? Promise.resolve(false) : ElMessageBox.confirm(
          fixConfirmText(plan, usb(), version),
          usb().fixTitle,
          { confirmButtonText: usb().fixStart, cancelButtonText: usb().cancel, type: 'warning', danger: Boolean(plan.replacesExternal) },
        ).then(() => true, () => false)),
        endStream: async () => {
          try { await sunshine.stopAllSessions(); return { ok: true } } catch (error) { return { ok: false, error: String(error) } }
        },
        // only the devices the person was told about; a new one makes the question come again before this step (usbipFix.js)
        detach: async (ports) => {
          const released = []
          for (const port of ports) {
            const result = await usbip.detach(port)
            if (!result?.success) return { ok: false, error: String(result?.message), raw: released.join('\n') }
            released.push(`port ${port}: released`)
          }
          usbipStatusChannel.invalidate()
          // the replacement refuses while the status still lists a device: read it again before the next step
          const read = await refreshStatus({ force: true, afterCurrent: true })
          if (read !== 'ok') return { ok: false, error: 'USB/IP status refresh failed after the devices were released', raw: released.join('\n') }
          return { ok: true, raw: released.join('\n') }
        },
        removeStale: () => removeStaleRaw({ confirmed: true, quiet: true }),
        cleanup: () => cleanupRaw({ confirmed: true, quiet: true }),
        install: () => installRaw({ confirmed: true, quiet: true }),
        replace: () => replaceRaw({ confirmed: true, quiet: true }),
        verify: verifyRepair,
        onReboot: ({ plan }) => saveFixResume(globalThis.localStorage, plan),
        sleep: (ms) => new Promise((resolve) => setTimeout(resolve, ms)),
        now: () => Date.now(),
      }, progress, { resume })
    } catch (error) {
      message?.close()
      throw error
    } finally {
      fixing.value = false
    }
    await refreshStatus({ force: true, afterCurrent: true })
    fixRaw.value = usbipFixRawText(finished, status)
    // a restart keeps the plan for the next run of the panel (onReboot above); everything else ends it
    if (!finished.outcome.startsWith('reboot-')) clearFixResume(globalThis.localStorage)
    const failedCleanup = ['cleanup', 'remove-stale'].includes(finished.failedStep)
    const result = describeFixResult(finished, usb(), (raw) => (failedCleanup ? cleanupError(raw) : friendlyError(raw)), version)
    // the cleanup, install, replacement and removal reported their own failures; the failures the repair made itself are reported here
    if (finished.outcome === 'failed' && !finished.reported) reportError(finished.error)
    // declined: the progress sentence that waited behind the question goes away
    if (!result.text) return message?.close()
    const action = result.action === 'restart'
      ? { actionText: usb().fixRestartNow, onClick: () => { void restartWindows() } }
      : result.action === 'guide' ? { actionText: usb().fixGuideAction, onClick: () => { void showFixGuide(result.guide) } }
        : result.action === 'diagnostics' ? { actionText: usb().diagnosticDetails, onClick: () => { void showFixDiagnostics() } } : {}
    show(result.tone, { message: result.text, duration: result.tone === 'success' || result.tone === 'info' ? 8000 : 20000, ...action })
    if (result.action === 'restart') {
      ElMessage.warning({ title: result.text, duration: 0, standing: 'usbip-fix-restart', revision: finished.outcome, ...action })
    }
  })

  /**
   * Continues a repair that waited for a restart: once, from the first panel that runs after Windows started again. The record is claimed
   * only when the repair can start now, so a driver operation of another component that holds the lock only delays it (it is tried again
   * when the lock is free); the person confirmed this before the restart.
   */
  async function resumeFixAfterRestart() {
    if (disposed || operationBlocked() || fixing.value) return
    if (!bootMs) await readBoot()
    const kept = loadFixResume(globalThis.localStorage, { bootMs })
    if (!kept) return
    clearFixResume(globalThis.localStorage)
    await fixTransport({ resume: kept })
  }

  sharedOperation.onOtherChange(() => refreshStatus({ afterCurrent: true, force: true }))
  // A pending restart continuation may have been skipped while another
  // component's confirmation was open. Cancellation does not mark a change.
  watch(sharedOperation.busyElsewhere, (busy, wasBusy) => {
    if (wasBusy && !busy && !resumeStarted && (status.cleanup_resume || status.install_resume)) {
      void refreshStatus({ force: true })
    }
    if (wasBusy && !busy) void resumeFixAfterRestart()
  })
  // Entering the page reads the devices again: the shared answer may be minutes old, and the native side answers this from cheap probes.
  onMounted(async () => {
    const booting = readBoot()
    await refreshStatus({ force: true })
    await booting
    await resumeFixAfterRestart()
  })
  // KeepAlive 重新显示时用缓存立即校准，过期则静默重探。
  onActivated(() => refreshStatus({ force: true }))
  return { status, statusLoading, statusLoaded, statusProbeFailed, installing, cleaning, replacing, exporting, fixing, fixOffered, fixRaw, attachedDevices, needsCleanup, staleResidual, canReplaceLegacy, transportBusy, statusClass, statusTitle, friendlyError, notifyError, refreshStatus, installTransport, cleanupResidual, removeStale, exportDiagnostics, replaceLegacy, fixTransport, restartWindows, showFixDiagnostics, showFixGuide }
}
