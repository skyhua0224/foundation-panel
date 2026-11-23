import assert from 'node:assert/strict'
import test from 'node:test'
import { createRenderer, nextTick, ref } from 'vue'

globalThis.localStorage ??= { getItem: () => null, setItem: () => {}, removeItem: () => {} }
globalThis.document ??= { documentElement: {} }
globalThis.window ??= {
  addEventListener() {},
  removeEventListener() {},
  dispatchEvent() {},
  matchMedia: () => ({ addEventListener() {}, matches: false }),
  __TAURI_INTERNALS__: { transformCallback: () => 1, invoke: async () => ({}) },
}

const { emitUsbipErrorReport, formatUsbipFriendlyError, usbipErrorReportCode, useUsbTransport } = await import('./useUsbTransport.js')
const { usbipStatusChannel } = await import('./deviceStatusStore.js')
const { usbip } = await import('../tauri-adapter.js')
const { useDeviceComponentOperation } = await import('./useDeviceComponentOperation.js')
const { useVbCableComponent } = await import('./useVbCableComponent.js')

test('usbipErrorReportCode extracts structured codes and subcodes without user text', () => {
  assert.equal(
    usbipErrorReportCode('USBIP-SETUP-016: Windows driver policy blocked (exit code 1) [!!! Error 0xe0000247 in C:\\Users\\Alice\\file]'),
    'usbip-setup-016.0xe0000247',
  )
  assert.equal(
    usbipErrorReportCode('USBIP-SETUP-010: the USB/IP installer failed with exit code 1 [pnputil failed]'),
    'usbip-setup-010.exit-1',
  )
  assert.equal(
    usbipErrorReportCode('USBIP-SETUP-007: transport installation failed: USBIP-EXEC-006: hash mismatch'),
    'usbip-exec-006',
  )
  assert.equal(usbipErrorReportCode('unexpected failure with C:\\Users\\secret'), 'usbip-unknown')
})

test('formatUsbipFriendlyError unwraps USBIP-SETUP-007 to the underlying specific error code', () => {
  const usbTexts = {
    unknownError: 'fallback',
    errors: {
      'USBIP-SETUP-007': 'generic install failed',
      'USBIP-SETUP-015': 'reboot pending',
      'USBIP-EXEC-006': 'payload mismatch',
    },
  }
  assert.equal(
    formatUsbipFriendlyError('USBIP-SETUP-007: transport installation failed: USBIP-EXEC-006: missing', usbTexts),
    'payload mismatch',
  )
  assert.equal(
    formatUsbipFriendlyError('USBIP-SETUP-015: a Windows restart is pending', usbTexts),
    'reboot pending',
  )
  assert.equal(formatUsbipFriendlyError('USBIP-SETUP-007: raw error', usbTexts), 'generic install failed')
  assert.equal(
    formatUsbipFriendlyError('USBIP-SETUP-018: another driver operation', usbTexts, { operationBusy: 'driver busy' }),
    'driver busy',
  )
  assert.equal(formatUsbipFriendlyError('other', usbTexts), 'fallback')
})

test('emitUsbipErrorReport sends only component, code and severity', () => {
  const sent = []
  const code = emitUsbipErrorReport(
    'USBIP-SETUP-015: pending restart (exit code 3010) [C:\\Users\\bob\\secret]',
    (event) => sent.push(event),
  )
  assert.equal(code, 'usbip-setup-015.exit-3010')
  assert.deepEqual(sent, [{ component: 'usbip', code: 'usbip-setup-015.exit-3010', severity: 'error' }])
})

test('a status response arriving after unmount cannot start automatic maintenance', async (t) => {
  const originalFetch = usbipStatusChannel.fetch
  const originals = { cleanupTransport: usbip.cleanupTransport, installTransport: usbip.installTransport, removeStaleNodes: usbip.removeStaleNodes }
  let calls = 0
  for (const name of Object.keys(originals)) usbip[name] = async () => { calls += 1; return { success: true, data: {} } }
  t.after(() => { usbipStatusChannel.fetch = originalFetch; Object.assign(usbip, originals) })
  const renderer = createRenderer({
    patchProp() {}, insert() {}, remove() {}, createElement: () => ({}), createText: () => ({}),
    createComment: () => ({}), setText() {}, setElementText() {}, parentNode: () => null, nextSibling: () => null,
  })
  for (const pending of [
    { cleanup_resume: true, cleanup_resume_op: 'cleanup', vhci_residual: true },
    { cleanup_resume: true, cleanup_resume_op: 'stale', vhci_residual: true, residual_kind: 'stale' },
    { install_resume: true, vhci_residual: false },
  ]) {
    let answer
    usbipStatusChannel.fetch = () => new Promise(resolve => { answer = resolve })
    const app = renderer.createApp({ setup() { useUsbTransport(); return () => null } })
    app.mount({})
    assert.equal(typeof answer, 'function')
    app.unmount()
    answer({ success: true, data: { supported: true, externally_managed: false, attached_devices: [], ...pending } })
    await new Promise(resolve => setImmediate(resolve))
    assert.equal(calls, 0)
  }
})

test('automatic maintenance queued before unmount does not start afterwards', async (t) => {
  const originalFetch = usbipStatusChannel.fetch
  const originalCleanup = usbip.cleanupTransport
  const originalQueue = globalThis.queueMicrotask
  const queued = []
  let calls = 0
  usbipStatusChannel.fetch = async () => ({ success: true, data: {
    supported: true, cleanup_resume: true, cleanup_resume_op: 'cleanup', externally_managed: false,
    vhci_residual: true, attached_devices: [],
  } })
  usbip.cleanupTransport = async () => { calls += 1; return { success: true, data: {} } }
  globalThis.queueMicrotask = callback => queued.push(callback)
  t.after(() => { usbipStatusChannel.fetch = originalFetch; usbip.cleanupTransport = originalCleanup; globalThis.queueMicrotask = originalQueue })
  const renderer = createRenderer({
    patchProp() {}, insert() {}, remove() {}, createElement: () => ({}), createText: () => ({}),
    createComment: () => ({}), setText() {}, setElementText() {}, parentNode: () => null, nextSibling: () => null,
  })
  let transport
  const app = renderer.createApp({ setup() { transport = useUsbTransport(); return () => null } })
  app.mount({})
  await transport.refreshStatus({ force: true })
  assert.equal(queued.length, 1)
  app.unmount()
  for (const callback of queued) callback()
  await new Promise(resolve => setImmediate(resolve))
  assert.equal(calls, 0)
})

test('automatic maintenance waiting for a refresh stops if its component unmounts', async (t) => {
  const originalFetch = usbipStatusChannel.fetch
  const originalCleanup = usbip.cleanupTransport
  const originalQueue = globalThis.queueMicrotask
  let calls = 0
  let queued = 0
  let app
  usbipStatusChannel.fetch = async () => ({ success: true, data: {
    supported: true, cleanup_resume: true, cleanup_resume_op: 'cleanup', externally_managed: false,
    vhci_residual: true, attached_devices: [],
  } })
  usbip.cleanupTransport = async () => { calls += 1; return { success: true, data: {} } }
  globalThis.queueMicrotask = callback => { queued += 1; callback(); app.unmount() }
  t.after(() => { usbipStatusChannel.fetch = originalFetch; usbip.cleanupTransport = originalCleanup; globalThis.queueMicrotask = originalQueue })
  const renderer = createRenderer({
    patchProp() {}, insert() {}, remove() {}, createElement: () => ({}), createText: () => ({}),
    createComment: () => ({}), setText() {}, setElementText() {}, parentNode: () => null, nextSibling: () => null,
  })
  app = renderer.createApp({ setup() { useUsbTransport(); return () => null } })
  app.mount({})
  await new Promise(resolve => setImmediate(resolve))
  assert.equal(queued, 1)
  assert.equal(calls, 0)
})

test('automatic USB/IP cleanup retries after another driver operation releases the guard', async (t) => {
  const originalFetch = usbipStatusChannel.fetch
  const originalCleanup = usbip.cleanupTransport
  let cleanupCalls = 0
  let firstAttempt
  let completed
  const first = new Promise(resolve => { firstAttempt = resolve })
  const done = new Promise(resolve => { completed = resolve })
  usbipStatusChannel.fetch = async () => ({ success: true, data: {
    supported: true, cleanup_resume: cleanupCalls < 2, cleanup_resume_op: 'cleanup', externally_managed: false,
    vhci_residual: true, attached_devices: [],
  } })
  usbip.cleanupTransport = async () => {
    cleanupCalls += 1
    if (cleanupCalls === 1) {
      firstAttempt()
      return { success: false, message: 'USBIP-SETUP-018: another driver operation' }
    }
    completed()
    return { success: true, data: { cleanup_resume: false, vhci_residual: false } }
  }
  const renderer = createRenderer({
    patchProp() {}, insert() {}, remove() {}, createElement: () => ({}), createText: () => ({}),
    createComment: () => ({}), setText() {}, setElementText() {}, parentNode: () => null, nextSibling: () => null,
  })
  const app = renderer.createApp({ setup() { useUsbTransport(); return () => null } })
  app.mount({})
  t.after(() => { app.unmount(); usbipStatusChannel.fetch = originalFetch; usbip.cleanupTransport = originalCleanup })
  await first
  assert.equal(cleanupCalls, 1)
  let deadline
  t.after(() => clearTimeout(deadline))
  await Promise.race([done, new Promise((_, reject) => { deadline = setTimeout(() => reject(new Error('cleanup did not retry')), 15000) })])
  clearTimeout(deadline)
  assert.equal(cleanupCalls, 2)
})

test('a pending USB/IP cleanup resumes when a VB-CABLE confirmation is cancelled', async (t) => {
  const originalFetch = usbipStatusChannel.fetch
  const originalCleanup = usbip.cleanupTransport
  let cleanupCalls = 0
  let cleanupStarted
  const cleanup = new Promise(resolve => { cleanupStarted = resolve })
  usbipStatusChannel.fetch = async () => ({ success: true, data: {
    supported: true, cleanup_resume: true, cleanup_resume_op: 'cleanup', externally_managed: false,
    vhci_residual: true, attached_devices: [],
  } })
  usbip.cleanupTransport = async () => {
    cleanupCalls += 1
    cleanupStarted()
    return { success: true, data: { cleanup_resume: false, vhci_residual: false } }
  }

  let cancel
  const vb = useVbCableComponent({
    api: { check: async () => ({ success: true, data: { state: 'disabled', archive_ready: false } }) },
    text: ref({}), shared: useDeviceComponentOperation(),
    confirm: () => new Promise((_, reject) => { cancel = reject }), invalidate: () => {},
  })
  await vb.check()
  const confirmation = vb.changeDriver('enable')
  await nextTick()

  let transport
  const renderer = createRenderer({
    patchProp() {}, insert() {}, remove() {}, createElement: () => ({}), createText: () => ({}),
    createComment: () => ({}), setText() {}, setElementText() {}, parentNode: () => null, nextSibling: () => null,
  })
  const app = renderer.createApp({ setup() { transport = useUsbTransport(); return () => null } })
  app.mount({})
  t.after(() => { app.unmount(); usbipStatusChannel.fetch = originalFetch; usbip.cleanupTransport = originalCleanup })
  await transport.refreshStatus({ force: true })
  assert.equal(cleanupCalls, 0)
  cancel(new Error('cancelled'))
  await confirmation
  await Promise.race([cleanup, new Promise((_, reject) => setTimeout(() => reject(new Error('cleanup did not resume')), 1000))])
  assert.equal(cleanupCalls, 1)
})

// ---- the one-click fix, through the real composable with the host and the driver stood in for ----
async function mountTransport(t, initial, { storage = null, onErrorEvent, sessions = [{ session_id: 1 }], boot = Math.floor(Date.now() / 1000) - 3600 } = {}) {
  const { sunshine, dualsense } = await import('../tauri-adapter.js')
  const { useDialogs } = await import('foundation-ui/dialog')
  const { useNotify } = await import('foundation-ui/notify')
  const { useI18n } = await import('../desktop/i18n/index.js')
  const world = { status: { supported: true, installed: true, version_valid: true, ready: false, externally_managed: false, vhci_residual: false, error_code: 'USBIP-EXEC-006', detail: 'USBIP-EXEC-006: the shared transport does not match the tested component combination', attached_devices: [], ...initial }, calls: [], sessions, dualsense: { state: 'ready', transport: { signature: 'signed' } }, signature: 'signed', boot }
  const savedStorage = globalThis.localStorage
  if (storage) globalThis.localStorage = storage
  const saved = { fetch: usbipStatusChannel.fetch, cleanup: usbip.cleanupTransport, install: usbip.installTransport, replace: usbip.replaceLegacyTransport, restart: usbip.restartWindows, detach: usbip.detach, boot: usbip.bootSecs, signature: usbip.driverSignature, dualsense: dualsense.getStatus, ask: sunshine.askActiveSessions, stop: sunshine.stopAllSessions }
  usbipStatusChannel.fetch = async () => ({ success: true, data: { ...world.status } })
  sunshine.askActiveSessions = async () => world.sessions
  sunshine.stopAllSessions = async () => { world.calls.push('stop-stream'); world.sessions = [] }
  usbip.cleanupTransport = async () => { world.calls.push('cleanup'); return world.cleanup ? world.cleanup() : { success: true, data: (world.status = { ...world.status, installed: false, error_code: '', detail: '' }) } }
  usbip.installTransport = async () => { world.calls.push('install'); return world.install ? world.install() : { success: true, data: (world.status = { ...world.status, installed: true, ready: true, error_code: '', detail: '' }) } }
  usbip.restartWindows = async () => { world.calls.push('restart-windows'); return { success: true } }
  usbip.replaceLegacyTransport = async () => { world.calls.push('replace'); return world.replace ? world.replace() : { success: true, data: (world.status = { ...world.status, externally_managed: false, replaceable: false, version_valid: true, ready: true, error_code: '', detail: '' }) } }
  usbip.detach = async (port) => { world.calls.push(`detach-${port}`); world.status = { ...world.status, attached_devices: world.status.attached_devices.filter((d) => d.port !== port) }; return { success: true } }
  dualsense.getStatus = async () => ({ success: true, data: world.dualsense })
  usbip.bootSecs = async () => ({ success: true, data: world.boot })
  usbip.driverSignature = async () => ({ success: true, data: world.signature })
  t.after(() => {
    usbipStatusChannel.fetch = saved.fetch
    usbip.cleanupTransport = saved.cleanup
    usbip.installTransport = saved.install
    usbip.replaceLegacyTransport = saved.replace
    usbip.restartWindows = saved.restart
    usbip.detach = saved.detach
    usbip.bootSecs = saved.boot
    usbip.driverSignature = saved.signature
    dualsense.getStatus = saved.dualsense
    globalThis.localStorage = savedStorage
    sunshine.askActiveSessions = saved.ask
    sunshine.stopAllSessions = saved.stop
  })
  const renderer = createRenderer({
    patchProp() {}, insert() {}, remove() {}, createElement: () => ({}), createText: () => ({}),
    createComment: () => ({}), setText() {}, setElementText() {}, parentNode: () => null, nextSibling: () => null,
  })
  let transport
  const app = renderer.createApp({ setup() { transport = useUsbTransport({ onErrorEvent }); return () => null } })
  app.mount({})
  let mounted = true
  const unmount = () => { if (mounted) { mounted = false; app.unmount() } }
  t.after(unmount)
  await transport.refreshStatus({ force: true })
  const until = async (read) => {
    for (let i = 0; i < 400; i += 1) { const value = read(); if (value) return value; await new Promise((resolve) => setTimeout(resolve, 5)) }
    throw new Error('timed out')
  }
  const dialogs = useDialogs()
  const notify = useNotify()
  const snack = () => notify.state.queue.find((item) => item.id === 'usbip-fix')
  const answer = async (action) => { const dialog = await until(() => dialogs.state.queue[0]); const asked = { ...dialog }; dialogs.settle(dialog.id, action); return asked }
  return { world, transport, unmount, until, snack, answer, texts: () => useI18n().t.value.deviceHub.usb, notify, dialogs }
}

test('the one-click fix ends the stream, cleans and installs in that order after one question, and reports at the bottom', async (t) => {
  const { world, transport, until, snack, answer, texts } = await mountTransport(t)
  assert.equal(transport.fixOffered.value, true, 'the notice of the transport carries the fix while the status reports the mismatch')
  const running = transport.fixTransport()
  const question = await answer('confirm')
  assert.ok(question.message.startsWith(texts().fixEndsStream), 'the question says the stream ends')
  assert.equal(question.title, texts().fixTitle)
  await running
  assert.deepEqual(world.calls, ['stop-stream', 'cleanup', 'install'])
  const done = await until(() => snack()?.timeout > 0 && snack())
  assert.ok(done.message.includes('USB/IP'), done.message)
  assert.equal(done.action, '')
  assert.equal(transport.fixing.value, false)
  assert.equal(transport.status.ready, true)
  assert.match(transport.fixRaw.value, /end-stream: ok\ncleanup: ok\ninstall: ok/)
})

test('a failed step shows its reason with the raw diagnostics one touch away, and the install is not attempted', async (t) => {
  const { world, transport, until, snack, answer, texts, dialogs } = await mountTransport(t)
  world.sessions = []
  world.cleanup = () => ({ success: false, message: 'USBIP-CLEAN-008: HostController busy (raw detail)' })
  const running = transport.fixTransport()
  assert.equal((await answer('confirm')).message.includes(texts().fixEndsStream), false, 'no stream, nothing about ending one')
  await running
  assert.deepEqual(world.calls, ['cleanup'])
  const failed = await until(() => snack()?.timeout > 0 && snack())
  assert.ok(failed.message.includes(texts().fixStepClean))
  assert.ok(failed.message.includes(texts().errors['USBIP-CLEAN-008']))
  assert.equal(failed.action, texts().diagnosticDetails)
  assert.match(transport.fixRaw.value, /cleanup: failed\nUSBIP-CLEAN-008: HostController busy \(raw detail\)/)
  failed.onAction()
  const raw = await answer('confirm')
  assert.equal(raw.title, texts().diagnosticDetails)
  assert.match(raw.message, /raw detail/)
  assert.equal(raw.cancel, false)
  assert.equal(dialogs.state.queue.length, 0)
})

test('declining the question leaves the system and the bottom message as they were', async (t) => {
  const { world, transport, snack, answer } = await mountTransport(t)
  const running = transport.fixTransport()
  await answer('cancel')
  await running
  assert.deepEqual(world.calls, [])
  await new Promise((resolve) => setTimeout(resolve, 20))
  assert.equal(snack(), undefined, 'the progress sentence that waited behind the question is gone')
  assert.equal(transport.fixing.value, false)
})

test('a restart that the cleanup or the install asks for is offered, and Windows restarts only after a second answer', async (t) => {
  const { world, transport, until, snack, answer, texts, notify } = await mountTransport(t)
  world.sessions = []
  world.install = () => ({ success: true, data: (world.status = { ...world.status, installed: true, ready: false, reboot_recommended: true, error_code: '', detail: '' }) })
  const running = transport.fixTransport()
  await answer('confirm')
  await running
  const offer = await until(() => snack()?.timeout > 0 && snack())
  assert.equal(offer.action, texts().fixRestartNow)
  assert.ok(Object.values(notify.state.systems).some((item) => item.actions?.[0]?.label === texts().fixRestartNow), 'the offer also stays in the status of the window')
  offer.onAction()
  assert.equal(world.calls.includes('restart-windows'), false, 'nothing restarts before the person says so')
  const second = await answer('confirm')
  assert.equal(second.message, texts().fixRestartConfirm)
  await until(() => world.calls.includes('restart-windows'))
})

test('only the version mismatch toast carries the fix, and not for an installation the person manages', async (t) => {
  const { transport, notify, until, texts } = await mountTransport(t)
  let message
  const toast = () => notify.state.queue.find((item) => item.message === message)
  message = transport.friendlyError('USBIP-EXEC-006: the shared transport does not match')
  transport.notifyError('USBIP-EXEC-006: the shared transport does not match')
  assert.equal((await until(toast)).action, texts().fixAction)
  notify.state.queue.length = 0
  message = transport.friendlyError('USBIP-EXEC-004: refused')
  transport.notifyError('USBIP-EXEC-004: refused')
  assert.equal((await until(toast)).action, '')
})

test('an installation the person manages that the host will not replace is not touched: no question, and the steps to remove it by hand', async (t) => {
  const { world, transport, notify, until, texts, dialogs, answer } = await mountTransport(t, { externally_managed: true, replaceable: false, version_valid: false })
  assert.equal(transport.fixOffered.value, true, 'the button leads to the steps')
  await transport.fixTransport()
  assert.equal(dialogs.state.queue.length, 0, 'no question: nothing will be changed')
  assert.deepEqual(world.calls, [])
  const blocked = await until(() => notify.state.queue.find((item) => item.id === 'usbip-fix' && item.timeout > 0))
  assert.equal(blocked.message, texts().externalDecision)
  assert.equal(blocked.action, texts().fixGuideAction)
  blocked.onAction()
  const guide = await answer('cancel')
  assert.ok(guide.message.includes('usbip-win2') && guide.message.includes('0.9.8.1'), guide.message)
  assert.equal(guide.confirmButtonText, texts().diagnosticDetails)
})

test('refreshStatus says how the read went: ok, failed, or disposed', async (t) => {
  const { transport, unmount } = await mountTransport(t)
  assert.equal(await transport.refreshStatus({ force: true }), 'ok')
  usbipStatusChannel.fetch = async () => ({ success: false, message: 'probe down' })
  assert.equal(await transport.refreshStatus({ force: true }), 'failed')
  usbipStatusChannel.fetch = async () => { throw new Error('bridge gone') }
  assert.equal(await transport.refreshStatus({ force: true }), 'failed')
  unmount()
  assert.equal(await transport.refreshStatus({ force: true }), 'disposed')
})

test('the fix does not start from a cached status when the read fails, and says why', async (t) => {
  const { world, transport, until, snack, texts, dialogs } = await mountTransport(t)
  usbipStatusChannel.fetch = async () => ({ success: false, message: 'USBIP-EXEC-007: probe down' })
  await transport.fixTransport()
  assert.equal(dialogs.state.queue.length, 0, 'no question is asked on a status that was not read')
  assert.deepEqual(world.calls, [], 'nothing was stopped, cleaned or installed')
  const failed = await until(() => snack()?.timeout > 0 && snack())
  assert.ok(failed.message.includes(texts().fixStepCheck))
  assert.ok(failed.message.includes(texts().errors['USBIP-EXEC-007']), 'the reason is what the probe said, translated')
  assert.equal(failed.action, texts().diagnosticDetails)
  assert.match(transport.fixRaw.value, /check: failed\nUSB\/IP status refresh failed/)
})

test('the fix does not start on a page that was closed', async (t) => {
  const { world, transport, unmount, dialogs } = await mountTransport(t)
  unmount()
  await transport.fixTransport()
  assert.equal(dialogs.state.queue.length, 0)
  assert.deepEqual(world.calls, [])
  assert.match(transport.fixRaw.value, /check: failed\nUSB\/IP status refresh was abandoned/)
})

test('the repair says it is done only after the DualSense native channel agrees; one that still reports the transport fails the check', async (t) => {
  const { world, transport, until, snack, answer, texts } = await mountTransport(t)
  world.sessions = []
  world.dualsense = { state: 'usbip_unsigned', transport: { signature: 'invalid' } }
  const running = transport.fixTransport()
  await answer('confirm')
  await running
  assert.deepEqual(world.calls, ['cleanup', 'install'])
  const failed = await until(() => snack()?.timeout > 0 && snack())
  assert.ok(failed.message.includes(texts().fixStepVerify))
  assert.ok(failed.message.includes(texts().errors['USBIP-VERIFY-002']))
  assert.equal(failed.action, texts().fixGuideAction)
})

test('a host that does not answer cannot check the native channel: the transport is reported repaired, and the person is told to start Sunshine', async (t) => {
  const { world, transport, until, snack, answer, texts } = await mountTransport(t)
  world.sessions = []
  world.dualsense = { state: 'host_unreachable', transport: { signature: 'signed' } }
  const running = transport.fixTransport()
  await answer('confirm')
  await running
  const done = await until(() => snack()?.timeout > 0 && snack())
  assert.equal(done.message, texts().fixDoneHostDown)
})

test('a driver with a bad signature is offered the repair though the transport reads ready, and attached devices are released first', async (t) => {
  const { world, transport, answer, dialogs, until, snack, texts } = await mountTransport(t, { ready: true, error_code: '', detail: '', attached_devices: [{ port: 1 }, { port: 2 }] })
  assert.equal(transport.fixOffered.value, false, 'a healthy transport shows no button')
  world.signature = 'invalid'
  await transport.refreshStatus({ force: true })
  assert.equal(transport.fixOffered.value, true, 'the entry follows the same signature the repair reads')
  world.sessions = []
  const running = transport.fixTransport()
  const question = await answer('confirm')
  assert.ok(question.message.includes('(2)'), 'the question says how many devices are released')
  world.install = () => { world.signature = 'signed'; return { success: true, data: (world.status = { ...world.status, installed: true, ready: true, error_code: '', detail: '' }) } }
  await running
  assert.deepEqual(world.calls, ['detach-1', 'detach-2', 'cleanup', 'install'])
  assert.equal(dialogs.state.queue.length, 0)
  assert.equal((await until(() => snack()?.timeout > 0 && snack())).message, texts().fixDone)
})

test('a failure that software cannot finish gives the steps, and the raw diagnostics are one touch further', async (t) => {
  const { world, transport, until, snack, answer, texts } = await mountTransport(t, { installed: false, error_code: '', detail: '' })
  world.sessions = []
  world.install = () => ({ success: false, message: 'USBIP-SETUP-007: transport installation failed: USBIP-SETUP-016: Windows driver policy blocked (exit code 1) [0xe0000247]' })
  const running = transport.fixTransport()
  await answer('confirm')
  await running
  const failed = await until(() => snack()?.timeout > 0 && snack())
  assert.equal(failed.action, texts().fixGuideAction)
  assert.ok(failed.message.includes(texts().errors['USBIP-SETUP-016']))
  failed.onAction()
  const guide = await answer('confirm')
  assert.ok(guide.message.split('\n').length >= 4, 'numbered steps')
  assert.equal(guide.title, texts().fixTitle)
  const raw = await answer('confirm')
  assert.equal(raw.title, texts().diagnosticDetails)
  assert.match(raw.message, /0xe0000247/)
})

const keeping = () => {
  const map = new Map()
  return { map, getItem: (k) => map.get(k) ?? null, setItem: (k, v) => map.set(k, String(v)), removeItem: (k) => map.delete(k) }
}

test('a repair that waits for a restart keeps its plan, and the panel that starts afterwards continues without asking again', async (t) => {
  const storage = keeping()
  const first = await mountTransport(t, { installed: false, error_code: '', detail: '' }, { storage, sessions: [] })
  first.world.install = () => ({ success: true, data: (first.world.status = { ...first.world.status, installed: true, ready: false, reboot_recommended: true }) })
  const running = first.transport.fixTransport()
  await first.answer('confirm')
  await running
  const kept = JSON.parse(storage.getItem('foundation.usbip.fix.resume'))
  assert.deepEqual(kept.steps, ['install', 'verify'])
  first.unmount()
  // the panel starts again after Windows restarted: the record is older than this run
  kept.written = first.world.boot * 1000 - 5000
  storage.setItem('foundation.usbip.fix.resume', JSON.stringify(kept))
  const second = await mountTransport(t, { installed: false, ready: false, error_code: '', detail: '', reboot_recommended: false }, { storage, sessions: [] })
  await second.until(() => second.world.calls.includes('install'))
  assert.equal(second.dialogs.state.queue.length, 0, 'nothing is asked: the person confirmed this before the restart')
  await second.until(() => second.snack()?.timeout > 0 && second.snack())
  assert.equal(storage.getItem('foundation.usbip.fix.resume'), null, 'the record is dropped once it has run')
  assert.deepEqual(second.world.calls, ['install'])
})

test('a record of the same run of the panel is not a restart: nothing continues', async (t) => {
  const storage = keeping()
  storage.setItem('foundation.usbip.fix.resume', JSON.stringify({ id: 'not-installed', steps: ['install', 'verify'], ports: [], written: Date.now() + 60_000 }))
  const { world, dialogs } = await mountTransport(t, { installed: false, ready: false, error_code: '', detail: '' }, { storage })
  await new Promise((resolve) => setTimeout(resolve, 50))
  assert.deepEqual(world.calls, [])
  assert.equal(dialogs.state.queue.length, 0)
})

test('a full usbip-win2 with devices attached: the devices are released and the status read again before the replacement, which refuses to run beside them', async (t) => {
  const { world, transport, answer, until, snack, texts } = await mountTransport(t, { externally_managed: true, replaceable: true, version_valid: false, ready: false, error_code: 'USBIP-SETUP-003', detail: 'x', attached_devices: [{ port: 1 }] })
  world.sessions = []
  const running = transport.fixTransport()
  const question = await answer('confirm')
  assert.ok(question.message.includes(texts().replaceConfirm) && question.message.includes('(1)'))
  await running
  assert.deepEqual(world.calls, ['detach-1', 'replace'])
  assert.equal((await until(() => snack()?.timeout > 0 && snack())).message, texts().fixDone)
})

test('a host that does not answer about the streams stops the repair before the question: unknown is not "no stream"', async (t) => {
  const { world, transport, notify, until, texts, dialogs } = await mountTransport(t)
  const { sunshine } = await import('../tauri-adapter.js')
  const asked = sunshine.askActiveSessions
  sunshine.askActiveSessions = async () => { throw new Error('connection refused') }
  t.after(() => { sunshine.askActiveSessions = asked })
  await transport.fixTransport()
  assert.equal(dialogs.state.queue.length, 0)
  assert.deepEqual(world.calls, [])
  const stopped = await until(() => notify.state.queue.find((item) => item.id === 'usbip-fix' && item.timeout > 0))
  assert.ok(stopped.message.includes(texts().fixStreamUnknown), stopped.message)
}, { timeout: 30000 })

test('a page that is closed while the question is open changes nothing when the answer comes', async (t) => {
  const { world, transport, unmount, dialogs, until } = await mountTransport(t)
  world.sessions = []
  const running = transport.fixTransport()
  const dialog = await until(() => dialogs.state.queue[0])
  unmount()
  dialogs.settle(dialog.id, 'confirm')
  await running
  assert.deepEqual(world.calls, [], 'nothing was stopped, released, cleaned or installed')
})

test('a restart that was already waiting when the button was pressed is kept; the record is claimed only after Windows started again', async (t) => {
  const storage = keeping()
  const first = await mountTransport(t, { cleanup_pending_restart: true, vhci_residual: true, ready: false }, { storage })
  await first.transport.fixTransport()
  const kept = JSON.parse(storage.getItem('foundation.usbip.fix.resume'))
  assert.deepEqual([kept.id, kept.steps], ['pending-restart-cleanup', ['cleanup', 'verify']])
  assert.equal(first.world.calls.length, 0)
  first.unmount()
  // the same Windows session: a panel opened again does not claim it, and the record stays
  const same = await mountTransport(t, { installed: false, ready: false, error_code: '', detail: '' }, { storage })
  await new Promise((resolve) => setTimeout(resolve, 40))
  assert.deepEqual(same.world.calls, [])
  assert.ok(storage.getItem('foundation.usbip.fix.resume'), 'still kept')
  same.unmount()
  // Windows started after it: claimed. The cleanup is done, and the install that is left was not in the plan the person confirmed: it asks.
  const later = await mountTransport(t, { installed: false, ready: false, error_code: '', detail: '' }, { storage, sessions: [], boot: Math.floor(Date.now() / 1000) + 3600 })
  const question = await later.answer('cancel')
  assert.ok(question.message.length > 0)
  assert.deepEqual(later.world.calls, [])
  assert.equal(storage.getItem('foundation.usbip.fix.resume'), null, 'claimed once')
})

test('a stream that is running when the panel continues after the restart is ended only after the person says so', async (t) => {
  const storage = keeping()
  storage.setItem('foundation.usbip.fix.resume', JSON.stringify({ id: 'not-installed', steps: ['install', 'verify'], ports: [], written: Math.floor(Date.now() / 1000) * 1000 - 7_200_000 }))
  const { world, answer, until } = await mountTransport(t, { installed: false, ready: false, error_code: '', detail: '' }, { storage })
  const question = await answer('confirm')
  assert.deepEqual(world.calls.filter((c) => c === 'stop-stream'), [], 'the stream was still running when the question was shown')
  assert.ok(question.message.length > 0)
  await until(() => world.calls.includes('install'))
  assert.ok(world.calls.indexOf('stop-stream') < world.calls.indexOf('install'))
})

test('the driver operation of another component only delays a continuation after the restart: the record stays until the lock is free', async (t) => {
  const storage = keeping()
  const { useDeviceComponentOperation } = await import('./useDeviceComponentOperation.js')
  const other = useDeviceComponentOperation()
  let release
  const holding = other.wrap(() => new Promise((resolve) => { release = resolve }))()
  t.after(() => { release?.(); return holding })
  storage.setItem('foundation.usbip.fix.resume', JSON.stringify({ id: 'not-installed', steps: ['install', 'verify'], ports: [], written: Math.floor(Date.now() / 1000) * 1000 - 7_200_000 }))
  const { world, until } = await mountTransport(t, { installed: false, ready: false, error_code: '', detail: '' }, { storage })
  world.sessions = []
  await new Promise((resolve) => setTimeout(resolve, 60))
  assert.deepEqual(world.calls, [], 'blocked by the other operation')
  assert.ok(storage.getItem('foundation.usbip.fix.resume'), 'the record was not claimed while it could not run')
  release()
  await holding
  await until(() => world.calls.includes('install'))
  assert.equal(storage.getItem('foundation.usbip.fix.resume'), null)
})

test('a failure that the cleanup reported is not reported a second time; a failure of the repair itself is', async (t) => {
  const events = []
  const { world, transport, answer } = await mountTransport(t, {}, { onErrorEvent: (event) => events.push(event.code) })
  world.sessions = []
  world.cleanup = () => ({ success: false, message: 'USBIP-CLEAN-008: HostController in use' })
  const running = transport.fixTransport()
  await answer('confirm')
  await running
  assert.deepEqual(events, ['usbip-clean-008'], 'one event for one failure')
})
