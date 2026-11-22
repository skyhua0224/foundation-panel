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
