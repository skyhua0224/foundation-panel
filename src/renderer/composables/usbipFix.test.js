import assert from 'node:assert/strict'
import test from 'node:test'
import 'vue' // loaded before the shim below, as the other tests of this folder do: Vue must not see the fake document
import { DESKTOP_LOCALES } from '../desktop/i18n/merge.js'
import {
  clearFixResume,
  describeFixResult,
  FIX_GUIDE_IDS,
  FIX_GUIDE_KEYS,
  fixConfirmText,
  fixGuideText,
  fixProgressText,
  guideFor,
  initialFix,
  loadFixResume,
  planUsbipFix,
  primaryCode,
  reduceFix,
  runUsbipFix,
  saveFixResume,
  STREAM_STOP_WAIT_MS,
  usbipFixOffered,
  usbipFixOfferedByStatus,
  usbipFixRawText,
} from './usbipFix.js'
import { USBIP_FIX_INVENTORY } from './usbipFixInventory.js'
import { en } from '../desktop/i18n/en.js'
import { zh } from '../desktop/i18n/zh.js'
import { withUsbipOwnership } from '../desktop/i18n/usbipOwnership.js'

globalThis.localStorage ??= { getItem: () => null, setItem: () => {}, removeItem: () => {} }
globalThis.document ??= { documentElement: {} }
globalThis.window ??= {
  addEventListener() {},
  removeEventListener() {},
  dispatchEvent() {},
  matchMedia: () => ({ addEventListener() {}, matches: false }),
  __TAURI_INTERNALS__: { transformCallback: () => 1, invoke: async () => ({}) },
}
const { formatUsbipFriendlyError } = await import('./useUsbTransport.js')

const VERSION = '0.9.8.1'
const base = { supported: true, installed: true, version_valid: true, ready: true, externally_managed: false, replaceable: false, vhci_residual: false, attached_devices: [] }
const mismatch = { ...base, ready: false, error_code: 'USBIP-EXEC-006', detail: 'USBIP-EXEC-006: the shared transport does not match the tested component combination' }

test('the repair is offered for the failures of the transport itself, not for the exporter, bad input, or an install that fails inside another code', () => {
  for (const code of ['USBIP-EXEC-001', 'USBIP-EXEC-002', 'USBIP-EXEC-003', 'USBIP-EXEC-006', 'USBIP-EXEC-007', 'USBIP-SETUP-001', 'USBIP-SETUP-002', 'USBIP-SETUP-003', 'USBIP-SETUP-008']) {
    assert.equal(usbipFixOffered(`${code}: something`), true, code)
  }
  for (const text of ['USBIP-SETUP-007: transport installation failed: USBIP-EXEC-006: hash mismatch', 'USBIP-EXEC-004: the exporter refused', 'USBIP-EXEC-005: no device', 'USBIP-INPUT-001: bad host', 'USBIP-UAC-001: canceled', '']) {
    assert.equal(usbipFixOffered(text), false, text)
  }
  assert.equal(primaryCode('USBIP-SETUP-007: transport installation failed: USBIP-SETUP-016: blocked'), 'USBIP-SETUP-016')
  assert.equal(usbipFixOfferedByStatus(mismatch), true)
  assert.equal(usbipFixOfferedByStatus(base), false, 'a healthy transport has nothing to repair')
  assert.equal(usbipFixOfferedByStatus({ ...base, supported: false }), false)
  assert.equal(usbipFixOfferedByStatus({ ...base, ready: false }), false, 'an unexplained state has no repair to offer')
  assert.equal(usbipFixOfferedByStatus({ ...base, installed: false, ready: false }), true)
  assert.equal(usbipFixOfferedByStatus({ ...base, externally_managed: true, ready: false, version_valid: false }), true, 'an installation of the person gets the steps to remove it by hand')
})

test('every row of the inventory is planned, or explained with the guide that its error gets', () => {
  const ids = new Set()
  for (const row of USBIP_FIX_INVENTORY) {
    assert.ok(!ids.has(row.id), `${row.id} appears once`)
    ids.add(row.id)
    for (const field of ['codes', 'detect', 'repair', 'confirm', 'reboot', 'verify']) assert.ok(row[field] !== undefined && row[field] !== '', `${row.id}.${field}`)
    if (row.status) {
      const plan = planUsbipFix(row.status)
      assert.ok(plan.ids.includes(row.id), `${row.id}: the plan names its row (${plan.ids})`)
      assert.equal(plan.kind, row.expect.kind, `${row.id}: kind`)
      assert.deepEqual(plan.steps, row.expect.steps ?? plan.steps, `${row.id}: steps`)
      for (const field of ['reason', 'guide', 'replacesExternal']) if (field in row.expect) assert.equal(plan[field], row.expect[field], `${row.id}.${field}`)
    } else if ('error' in row) {
      assert.equal(guideFor(row.error), row.guide, `${row.id}: the guide of its error`)
    }
  }
  // the states the plan can produce are all in the table
  for (const id of ['not-installed', 'install-resume', 'damaged-install', 'old-version', 'mismatch-hash-abi', 'residual-broken', 'stale-controller', 'attached-ports', 'unsigned-driver', 'cleanup-resume', 'tool-failed',
    'external-incompatible-replaceable', 'external-incompatible-manual', 'external-stale-controller', 'external-compatible-healthy', 'pending-restart-cleanup', 'pending-restart-install', 'reboot-recommended',
    'healthy', 'unsupported-os', 'unexplained', 'stream-active', 'driver-op-busy', 'uac-cancelled', 'root-hub-rolled-back', 'root-hub-rollback-incomplete', 'usb-boot-disk', 'guard-could-not-run',
    'driver-policy-blocked', 'smart-app-control', 'removal-access-denied', 'removal-in-use', 'installer-generic-failure', 'replace-rolled-back', 'verify-failed', 'windows-10', 'host-not-running']) {
    assert.ok(ids.has(id), `the inventory has ${id}`)
  }
})

test('the codes of the Rust side each belong to a row of the inventory', () => {
  const listed = new Set(USBIP_FIX_INVENTORY.flatMap((row) => row.codes))
  const covered = (code) => listed.has(code) || (code.startsWith('USBIP-REPLACE-') && listed.has('USBIP-REPLACE-*')) || code.startsWith('USBIP-INPUT-') || ['USBIP-EXEC-004', 'USBIP-EXEC-005', 'USBIP-DIAG-001', 'USBIP-RESTART-001', 'USBIP-SETUP-007', 'USBIP-REPLACE-001', 'USBIP-REPLACE-002', 'USBIP-REPLACE-003', 'USBIP-REPLACE-004', 'USBIP-REPLACE-005', 'USBIP-CLEAN-005', 'USBIP-CLEAN-006'].includes(code)
  for (const code of ['USBIP-CLEAN-001', 'USBIP-CLEAN-002', 'USBIP-CLEAN-003', 'USBIP-CLEAN-004', 'USBIP-CLEAN-006', 'USBIP-CLEAN-007', 'USBIP-CLEAN-008', 'USBIP-EXEC-001', 'USBIP-EXEC-002', 'USBIP-EXEC-003', 'USBIP-EXEC-006', 'USBIP-EXEC-007',
    'USBIP-SETUP-001', 'USBIP-SETUP-002', 'USBIP-SETUP-003', 'USBIP-SETUP-004', 'USBIP-SETUP-005', 'USBIP-SETUP-006', 'USBIP-SETUP-008', 'USBIP-SETUP-010', 'USBIP-SETUP-011', 'USBIP-SETUP-012', 'USBIP-SETUP-013', 'USBIP-SETUP-014',
    'USBIP-SETUP-015', 'USBIP-SETUP-016', 'USBIP-SETUP-017', 'USBIP-SETUP-018', 'USBIP-UAC-001', 'USBIP-UAC-002', 'USBIP-UAC-003', 'USBIP-REPLACE-002', 'USBIP-REPLACE-003']) {
    assert.ok(covered(code), `${code} is in a row`)
  }
})

test('a full usbip-win2 of the person is replaced only when the host allows it, and never planned away behind the person\'s back', () => {
  const replace = planUsbipFix({ ...base, ready: false, externally_managed: true, replaceable: true, version_valid: false, attached_devices: [{ port: 1 }, { port: 2 }] })
  assert.deepEqual([replace.kind, replace.steps, replace.replacesExternal, replace.attachedCount], ['replace', ['detach', 'replace', 'verify'], true, 2])
  for (const status of [
    { ...mismatch, externally_managed: true, replaceable: false },
    { ...mismatch, externally_managed: true, replaceable: false, vhci_residual: true },
    { ...base, ready: false, externally_managed: true, replaceable: false, version_valid: false },
  ]) {
    const plan = planUsbipFix(status)
    assert.deepEqual([plan.kind, plan.reason, plan.guide, plan.steps], ['blocked', 'external', 'external', []])
  }
})

test('attached devices are released first, before anything is cleaned or replaced, and only when a step needs the transport free', () => {
  const devices = [{ port: 1 }]
  assert.deepEqual(planUsbipFix({ ...mismatch, attached_devices: devices }).steps, ['detach', 'cleanup', 'install', 'verify'])
  assert.deepEqual(planUsbipFix({ ...base, ready: false, vhci_residual: true, residual_kind: 'stale', attached_devices: devices }).steps, ['remove-stale', 'verify'], 'a live controller stays: nothing attached is touched')
  assert.deepEqual(planUsbipFix({ ...base, attached_devices: devices }).kind, 'healthy', 'attached devices on a healthy transport are the person\'s own')
})

test('the reducer walks check, plan, question, steps and an end, and keeps the log of every step', () => {
  let state = reduceFix(initialFix(), { type: 'check' })
  assert.equal(state.phase, 'checking')
  state = reduceFix(state, { type: 'plan', plan: { kind: 'clean-install', steps: ['cleanup', 'install'] } })
  assert.deepEqual([state.phase, state.steps], ['confirming', ['cleanup', 'install']])
  state = reduceFix(state, { type: 'confirmed' })
  assert.deepEqual([state.phase, state.index], ['running', 0])
  state = reduceFix(state, { type: 'step-done', step: 'cleanup', raw: 'cleaned' })
  assert.equal(state.index, 1)
  state = reduceFix(state, { type: 'step-failed', step: 'install', error: 'USBIP-SETUP-010: failed' })
  assert.deepEqual([state.phase, state.outcome, state.failedStep], ['finished', 'failed', 'install'])
  assert.deepEqual(state.log.map((entry) => [entry.step, entry.ok]), [['cleanup', true], ['install', false]])
  assert.equal(reduceFix(initialFix(), { type: 'unknown' }).phase, 'idle')
  for (const [plan, outcome] of [
    [{ kind: 'healthy', steps: [] }, 'healthy'],
    [{ kind: 'blocked', reason: 'external', steps: [] }, 'blocked-external'],
    [{ kind: 'reboot', reason: 'cleanup', steps: [] }, 'reboot-cleanup'],
  ]) assert.equal(reduceFix(reduceFix(initialFix(), { type: 'check' }), { type: 'plan', plan }).outcome, outcome)
  assert.equal(reduceFix(state, { type: 'declined' }).outcome, 'cancelled')
})

/** Fake world: records the order of the calls and lets each test choose the answers. */
function world(overrides = {}) {
  const calls = []
  let clock = 0
  let sessions = overrides.sessions === undefined ? [{ session_id: 1 }] : overrides.sessions
  const answer = (name, fallback) => async () => {
    calls.push(name)
    const value = overrides[name]
    if (value instanceof Error) throw value
    return typeof value === 'function' ? value() : value ?? fallback
  }
  const deps = {
    refresh: async () => { calls.push('refresh'); return overrides.status || mismatch },
    sessions: async () => {
      calls.push('sessions')
      if (sessions instanceof Error) throw sessions
      const now = sessions
      if (overrides.streamEndsAfter !== undefined && calls.filter((c) => c === 'sessions').length > overrides.streamEndsAfter) sessions = []
      return now
    },
    confirm: async (question) => { calls.push('confirm'); world.questions.push(question); return overrides.confirm ?? true },
    endStream: answer('endStream', { ok: true }),
    detach: answer('detach', { ok: true }),
    removeStale: answer('removeStale', { ok: true, data: { vhci_residual: false } }),
    verify: answer('verify', { ok: true }),
    cleanup: answer('cleanup', { ok: true, data: { vhci_residual: false } }),
    install: answer('install', { ok: true, data: { ready: true } }),
    replace: answer('replace', { ok: true, data: { ready: true } }),
    sleep: async (ms) => { clock += ms },
    now: () => clock,
  }
  world.questions = []
  return { deps, calls }
}
world.questions = []

test('a live stream is ended first, then the shared transport is cleaned and the bundled one installed, after one question', async () => {
  const { deps, calls } = world({ streamEndsAfter: 2 })
  const seen = []
  const state = await runUsbipFix(deps, (next) => seen.push(`${next.phase}:${next.index}`))
  assert.equal(state.outcome, 'done')
  assert.deepEqual(calls.filter((c) => c !== 'sessions'), ['refresh', 'confirm', 'refresh', 'endStream', 'cleanup', 'install', 'verify'], 'the system is read again after the answer')
  assert.equal(calls.filter((c) => c === 'confirm').length, 1, 'the person is asked once')
  assert.ok(calls.indexOf('endStream') < calls.indexOf('cleanup'), 'the stream ends before the transport changes')
  assert.equal(world.questions.at(-1).streaming, true)
  assert.deepEqual(state.steps, ['end-stream', 'cleanup', 'install', 'verify'])
  assert.deepEqual(seen.slice(0, 3), ['checking:0', 'confirming:0', 'running:0'])
  assert.deepEqual(state.log.map((entry) => entry.step), ['end-stream', 'cleanup', 'install', 'verify'])
})

test('without a stream nothing is ended and the question does not mention one', async () => {
  const { deps, calls } = world({ sessions: [] })
  const state = await runUsbipFix(deps)
  assert.equal(state.outcome, 'done')
  assert.equal(calls.includes('endStream'), false)
  assert.equal(world.questions.at(-1).streaming, false)
  assert.deepEqual(state.steps, ['cleanup', 'install', 'verify'])
})

test('a host that cannot say whether a stream runs is not "no stream": the repair stops before the question and changes nothing', async () => {
  for (const unreadable of [new Error('host unreachable'), null]) {
    const { deps, calls } = world({ sessions: unreadable })
    deps.sessions = async () => { calls.push('sessions'); if (unreadable) throw unreadable; return null }
    const state = await runUsbipFix(deps)
    assert.deepEqual([state.outcome, state.failedStep], ['failed', 'check'])
    assert.match(state.error, /^USBIP-SETUP-006: unable to verify stream ownership/)
    for (const step of ['confirm', 'endStream', 'detach', 'removeStale', 'cleanup', 'install', 'replace', 'verify']) assert.equal(calls.includes(step), false, `${step} was not called`)
    assert.ok(calls.filter((c) => c === 'sessions').length > 3, 'it asked again for a while')
    assert.equal(describeFixResult(state, usb.en, friendly(usb.en), VERSION).text.includes(usb.en.fixStreamUnknown), true)
  }
})

test('a host that fails once and then answers with an empty list lets the repair go on', async () => {
  const { deps, calls } = scriptedSessions([[]], { first: [new Error('timeout')] })
  const state = await runUsbipFix(deps)
  assert.equal(state.outcome, 'done')
  assert.deepEqual(calls.filter((c) => ['cleanup', 'install'].includes(c)), ['cleanup', 'install'])
})

test('declining the question changes nothing', async () => {
  const { deps, calls } = world({ confirm: false })
  const state = await runUsbipFix(deps)
  assert.equal(state.outcome, 'cancelled')
  assert.deepEqual(calls.filter((c) => c !== 'sessions'), ['refresh', 'confirm'])
})

test('a stream that does not end in time fails the end-stream step with the stream code, before anything is cleaned', async () => {
  const { deps, calls } = world({ streamEndsAfter: 10_000 })
  const state = await runUsbipFix(deps)
  assert.equal(state.outcome, 'failed')
  assert.equal(state.failedStep, 'end-stream')
  assert.match(state.error, /^USBIP-SETUP-006:/)
  assert.equal(calls.includes('cleanup'), false)
  assert.ok(calls.filter((c) => c === 'sessions').length > 3)
  assert.ok(STREAM_STOP_WAIT_MS >= 10_000)
})

/**
 * A host whose answers to "which streams run" are scripted: first the answers of the initial look (`first`), then the stream that shows
 * at the plan and at the second look, then `answers` (what follows the stop request).
 */
function scriptedSessions(answers, { first = [] } = {}) {
  const { deps, calls } = world({ sessions: [] })
  const queue = [...first, [{ session_id: 1 }], [{ session_id: 1 }], ...answers]
  deps.sessions = async () => {
    calls.push('sessions')
    const next = queue.length > 1 ? queue.shift() : queue[0]
    if (next instanceof Error) throw next
    return next
  }
  return { deps, calls }
}

test('a stream query that cannot be read after the stop request is not "the stream ended": the wait goes on and stops at its limit', async () => {
  for (const unreadable of [new Error('host unreachable'), null]) {
    const { deps, calls } = scriptedSessions([unreadable])
    const state = await runUsbipFix(deps)
    assert.deepEqual([state.outcome, state.failedStep], ['failed', 'end-stream'])
    assert.match(state.error, /^USBIP-SETUP-006: unable to verify that the Sunshine stream ended/)
    assert.equal(calls.includes('cleanup'), false, 'the shared transport is not touched while a stream may still run')
    assert.equal(calls.includes('install'), false)
    assert.ok(calls.filter((c) => c === 'sessions').length > 3, 'it asked again, not once')
  }
})

test('a stream query that fails for a while and then answers with an empty list lets the fix go on', async () => {
  const { deps, calls } = scriptedSessions([new Error('timeout'), null, [], []])
  const state = await runUsbipFix(deps)
  assert.equal(state.outcome, 'done')
  assert.ok(calls.indexOf('cleanup') > calls.lastIndexOf('sessions'), 'the cleanup came after the empty answer')
})

test('a query that fails and then still shows a stream ends with the stream message, not the unreadable one', async () => {
  const { deps } = scriptedSessions([new Error('timeout'), [{ session_id: 1 }]])
  const state = await runUsbipFix(deps)
  assert.match(state.error, /^USBIP-SETUP-006: finish the active Sunshine stream/)
})

test('a host that refuses to end the stream fails that step with what it said', async () => {
  const { deps, calls } = world({ endStream: { ok: false, error: 'HTTP 500' } })
  const state = await runUsbipFix(deps)
  assert.deepEqual([state.outcome, state.failedStep, state.error], ['failed', 'end-stream', 'HTTP 500'])
  assert.equal(calls.includes('cleanup'), false)
  const thrown = world({ endStream: new Error('connection reset') })
  assert.equal((await runUsbipFix(thrown.deps)).failedStep, 'end-stream')
})

test('a failed cleanup stops the fix before the install and keeps the raw answer', async () => {
  const { deps, calls } = world({ sessions: [], cleanup: { ok: false, error: 'USBIP-CLEAN-008: the host controller is in use' } })
  const state = await runUsbipFix(deps)
  assert.deepEqual([state.outcome, state.failedStep], ['failed', 'cleanup'])
  assert.equal(calls.includes('install'), false)
  assert.match(usbipFixRawText(state, { detail: 'the status detail' }), /cleanup: failed\nUSBIP-CLEAN-008: the host controller is in use\nthe status detail/)
})

test('a cleanup that waits for a restart ends the fix with the restart offer, not with an install that cannot work', async () => {
  const { deps, calls } = world({ sessions: [], cleanup: { ok: true, data: { cleanup_pending_restart: true } } })
  const state = await runUsbipFix(deps)
  assert.equal(state.outcome, 'reboot-cleanup')
  assert.equal(calls.includes('install'), false)
  const residual = world({ sessions: [], cleanup: { ok: true, data: { vhci_residual: true } } })
  const left = await runUsbipFix(residual.deps)
  assert.deepEqual([left.outcome, left.failedStep], ['failed', 'cleanup'])
  assert.match(left.error, /^USBIP-CLEAN-002:/)
})

test('an install that needs a restart, or is not ready, is told apart from a finished one', async () => {
  const restart = await runUsbipFix(world({ sessions: [], install: { ok: true, data: { ready: false, reboot_recommended: true } } }).deps)
  assert.equal(restart.outcome, 'reboot-install')
  const pending = await runUsbipFix(world({ sessions: [], install: { ok: true, data: { ready: false, install_pending_restart: true } } }).deps)
  assert.equal(pending.outcome, 'reboot-install')
  const notReady = await runUsbipFix(world({ sessions: [], install: { ok: true, data: { ready: false, detail: 'USBIP-EXEC-006: still differs' } } }).deps)
  assert.deepEqual([notReady.outcome, notReady.failedStep, notReady.error], ['failed', 'install', 'USBIP-EXEC-006: still differs'])
})

test('a full usbip-win2 of the person is replaced only after the question, by the replacement and by nothing else', async () => {
  const status = { ...base, ready: false, externally_managed: true, replaceable: true }
  const { deps, calls } = world({ status, sessions: [] })
  const state = await runUsbipFix(deps)
  assert.equal(state.outcome, 'done')
  assert.deepEqual(calls.filter((c) => c !== 'sessions'), ['refresh', 'confirm', 'refresh', 'replace', 'verify'])
  assert.equal(world.questions.at(-1).plan.replacesExternal, true)
  const declined = world({ status, sessions: [], confirm: false })
  await runUsbipFix(declined.deps)
  assert.equal(declined.calls.includes('replace'), false)
})

test('an installation the person manages and the host will not replace is left alone and no question is asked', async () => {
  const { deps, calls } = world({ status: { ...mismatch, externally_managed: true, replaceable: false } })
  const state = await runUsbipFix(deps)
  assert.equal(state.outcome, 'blocked-external')
  assert.deepEqual(calls, ['refresh'])
})

test('a healthy status and a pending restart end before any question', async () => {
  for (const [status, outcome] of [
    [base, 'healthy'],
    [{ ...mismatch, cleanup_pending_restart: true }, 'reboot-cleanup'],
  ]) {
    const { deps, calls } = world({ status })
    assert.equal((await runUsbipFix(deps)).outcome, outcome)
    assert.equal(calls.includes('confirm'), false)
  }
})

test('a step that did not start because another driver operation runs, or that throws, fails with its reason', async () => {
  const busy = await runUsbipFix(world({ sessions: [], cleanup: { ok: false, skipped: true } }).deps)
  assert.deepEqual([busy.outcome, busy.failedStep], ['failed', 'cleanup'])
  assert.match(busy.error, /^USBIP-SETUP-018:/)
  const thrown = await runUsbipFix(world({ sessions: [], install: new Error('boom') }).deps)
  assert.deepEqual([thrown.outcome, thrown.failedStep], ['failed', 'install'])
  const unreadable = await runUsbipFix({ ...world().deps, refresh: async () => { throw new Error('probe down') } })
  assert.deepEqual([unreadable.outcome, unreadable.failedStep], ['failed', 'check'])
})

// ---- the words ----
// the tables as the window reads them: the language files with the ownership messages laid over them
const usb = { en: withUsbipOwnership(en, 'en').deviceHub.usb, zh: withUsbipOwnership(zh, 'zh').deviceHub.usb }
const friendly = (texts) => (raw) => formatUsbipFriendlyError(raw, texts)

test('error mapping: a failed step names the step and gives the translated reason of its code, in each language', () => {
  for (const [name, texts] of Object.entries(usb)) {
    const text = (state) => describeFixResult({ ...initialFix(), ...state }, texts, friendly(texts), VERSION)
    const cleanup = text({ outcome: 'failed', failedStep: 'cleanup', error: 'USBIP-CLEAN-008: busy' })
    assert.equal(cleanup.tone, 'error', name)
    assert.equal(cleanup.action, 'diagnostics', name)
    assert.ok(cleanup.text.includes(texts.fixStepClean), name)
    assert.ok(cleanup.text.includes(texts.errors['USBIP-CLEAN-008']), name)
    const install = text({ outcome: 'failed', failedStep: 'install', error: 'USBIP-SETUP-007: transport installation failed: USBIP-SETUP-016: blocked' })
    assert.ok(install.text.includes(texts.errors['USBIP-SETUP-016']), `${name}: the underlying code of an install is the reason`)
    assert.ok(install.text.includes(VERSION), `${name}: the install step names the version`)
    const stream = text({ outcome: 'failed', failedStep: 'end-stream', error: 'HTTP 500: stop failed' })
    assert.ok(stream.text.includes(texts.fixStopFailed), `${name}: a host answer without a code says the stream could not be ended`)
    const timeout = text({ outcome: 'failed', failedStep: 'end-stream', error: 'USBIP-SETUP-006: finish the active Sunshine stream' })
    assert.ok(timeout.text.includes(texts.errors['USBIP-SETUP-006']), name)
    const unreadable = text({ outcome: 'failed', failedStep: 'check', error: 'USB/IP status refresh failed: host did not answer' })
    assert.ok(unreadable.text.includes(texts.statusUnavailable), `${name}: a status that could not be read says so`)
    assert.ok(unreadable.text.includes(texts.fixStepCheck), name)
    const restart = text({ outcome: 'failed', failedStep: 'cleanup', error: 'USBIP-RESTART-001: no shutdown.exe' })
    assert.ok(restart.text.includes(texts.errors['USBIP-RESTART-001']), name)
  }
})

test('results: done and healthy are plain, a restart offers the restart, a refusal explains itself, a decline says nothing', () => {
  const texts = usb.en
  const text = (outcome) => describeFixResult({ ...initialFix(), outcome }, texts, friendly(texts), VERSION)
  assert.deepEqual(text('done'), { tone: 'success', text: texts.fixDone, action: '', guide: '' })
  assert.equal(text('healthy').tone, 'info')
  assert.equal(text('reboot-cleanup').action, 'restart')
  assert.equal(text('reboot-install').action, 'restart')
  assert.ok(text('reboot-install').text.includes(VERSION))
  assert.deepEqual([text('blocked-external').text, text('blocked-external').action, text('blocked-external').guide], [texts.externalDecision, 'guide', 'external'])
  assert.equal(text('blocked-nothing').action, 'diagnostics')
  assert.equal(text('cancelled').text, '')
})

test('the question and the progress sentence say what will happen', () => {
  const texts = usb.zh
  const clean = fixConfirmText({ kind: 'clean-install', streaming: false }, texts, VERSION)
  assert.ok(clean.includes(VERSION) && !clean.includes(texts.fixEndsStream))
  const streaming = fixConfirmText({ kind: 'clean-install', streaming: true }, texts, VERSION)
  assert.ok(streaming.startsWith(texts.fixEndsStream), 'the stream that will end is named first')
  assert.ok(fixConfirmText({ kind: 'replace', replacesExternal: true, steps: ['replace', 'verify'], streaming: false }, texts, VERSION).startsWith(texts.replaceConfirm), 'the replacement names the full usbip-win2 that goes')
  const state = { ...initialFix(), steps: ['end-stream', 'cleanup', 'install'], index: 1, phase: 'running' }
  assert.equal(fixProgressText(state, texts, VERSION), '修复中 2/3：清理共享传输…')
  assert.equal(fixProgressText({ ...state, index: 2 }, texts, VERSION), `修复中 3/3：安装 USB/IP ${VERSION}…`)
  assert.equal(fixProgressText({ ...initialFix() }, texts, VERSION), '')
})

test('every language has the words of the fix, with the placeholders its sentences need', async () => {
  const placeholders = {
    fixConfirm: ['{version}'],
    fixProgress: ['{n}', '{total}', '{step}'],
    fixStepInstall: ['{version}'],
    fixConfirmInstall: ['{version}'],
    fixWillDetach: ['{count}'],
    fixGuideExternal: ['{version}'],
    fixRebootInstall: ['{version}'],
    fixFailed: ['{step}', '{reason}'],
  }
  const keys = ['fixAction', 'fixTitle', 'fixConfirm', 'fixEndsStream', 'fixStart', 'fixProgress', 'fixStepCheck', 'fixStepStream', 'fixStepClean', 'fixStepInstall', 'fixStepReplace', 'fixDone', 'fixHealthy', 'fixFailed', 'fixRebootCleanup', 'fixRebootInstall', 'fixNothing', 'fixStopFailed', 'fixRestartNow', 'fixRestartConfirm',
    'fixConfirmInstall', 'fixConfirmStale', 'fixWillDetach', 'fixMayReboot', 'fixStepDetach', 'fixStepStale', 'fixStepVerify', 'fixDoneHostDown', 'fixGuideAction', 'fixResumeStart', ...Object.values(FIX_GUIDE_KEYS)]
  assert.equal(DESKTOP_LOCALES.length, 20)
  for (const locale of DESKTOP_LOCALES) {
    const table = locale === 'en' ? en : locale === 'zh' ? zh : (await import(`../desktop/i18n/${locale}.js`))[locale]
    const texts = table.deviceHub.usb
    for (const key of keys) {
      assert.equal(typeof texts[key], 'string', `${locale}.${key}`)
      assert.ok(texts[key].length > 0, `${locale}.${key}`)
    }
    for (const [key, list] of Object.entries(placeholders)) for (const token of list) assert.ok(texts[key].includes(token), `${locale}.${key} keeps ${token}`)
    for (const code of ['USBIP-RESTART-001', 'USBIP-VERIFY-001', 'USBIP-VERIFY-002']) assert.ok(texts.errors[code], `${locale} explains ${code}`)
    for (const id of FIX_GUIDE_IDS) assert.match(fixGuideText(id, texts, VERSION), /\n/, `${locale}: the guide ${id} is a list of steps`)
    assert.ok(!texts.fixConfirm.includes('0.9.8.1'), `${locale}: the version comes from the pinned contract, not from the text`)
  }
})

// ---- the steps beyond install and cleanup ----
test('attached devices are released before the cleanup, and a release that fails stops the repair there', async () => {
  const withDevices = { ...mismatch, attached_devices: [{ port: 1 }, { port: 2 }] }
  const { deps, calls } = world({ status: withDevices, sessions: [] })
  const state = await runUsbipFix(deps)
  assert.equal(state.outcome, 'done')
  assert.deepEqual(calls.filter((c) => c !== 'sessions'), ['refresh', 'confirm', 'refresh', 'detach', 'cleanup', 'install', 'verify'])
  assert.equal(world.questions.at(-1).plan.attachedCount, 2)
  const refused = world({ status: withDevices, sessions: [], detach: { ok: false, error: 'USBIP-UAC-001: administrator authorization was canceled' } })
  const failed = await runUsbipFix(refused.deps)
  assert.deepEqual([failed.outcome, failed.failedStep], ['failed', 'detach'])
  assert.equal(refused.calls.includes('cleanup'), false, 'the transport is not cleaned under an attached device')
})

test('extra controllers are removed alone: the working transport is not cleaned or reinstalled', async () => {
  const stale = { ...base, ready: false, vhci_residual: true, residual_kind: 'stale' }
  const { deps, calls } = world({ status: stale, sessions: [] })
  const state = await runUsbipFix(deps)
  assert.equal(state.outcome, 'done')
  assert.deepEqual(calls.filter((c) => c !== 'sessions'), ['refresh', 'confirm', 'refresh', 'removeStale', 'verify'])
  const waiting = await runUsbipFix(world({ status: stale, sessions: [], removeStale: { ok: true, data: { cleanup_pending_restart: true } } }).deps)
  assert.equal(waiting.outcome, 'reboot-cleanup')
})

test('the repair is done only after the check: a transport that is not ready, or a native channel that says so, fails the check step', async () => {
  const notReady = await runUsbipFix(world({ sessions: [], verify: { ok: false, error: 'USBIP-VERIFY-001: USB/IP is not ready after the repair' } }).deps)
  assert.deepEqual([notReady.outcome, notReady.failedStep], ['failed', 'verify'])
  assert.equal(describeFixResult(notReady, usb.en, friendly(usb.en), VERSION).guide, 'generic')
  const nativeChannel = await runUsbipFix(world({ sessions: [], verify: { ok: false, error: 'USBIP-VERIFY-002: the DualSense native channel reports usbip_unsigned' } }).deps)
  assert.equal(nativeChannel.failedStep, 'verify')
  const hostDown = await runUsbipFix(world({ sessions: [], verify: { ok: true, note: 'host-unreachable' } }).deps)
  assert.equal(hostDown.outcome, 'done')
  assert.equal(describeFixResult(hostDown, usb.zh, friendly(usb.zh), VERSION).text, usb.zh.fixDoneHostDown)
  assert.equal(describeFixResult(await runUsbipFix(world({ sessions: [] }).deps), usb.zh, friendly(usb.zh), VERSION).text, '已恢复，可以开始串流。')
})

test('the question names everything that will be ended, released or removed, and that a restart may follow', () => {
  const texts = usb.en
  const plan = { kind: 'replace', replacesExternal: true, steps: ['end-stream', 'detach', 'replace', 'verify'], streaming: true, attachedCount: 2 }
  const text = fixConfirmText(plan, texts, VERSION)
  assert.ok(text.includes(texts.fixEndsStream) && text.includes('(2)') && text.includes(texts.replaceConfirm) && text.endsWith(texts.fixMayReboot))
  assert.ok(fixConfirmText({ kind: 'install', steps: ['install', 'verify'] }, texts, VERSION).includes(`USB/IP ${VERSION}`))
  assert.ok(fixConfirmText({ kind: 'remove-stale', steps: ['remove-stale', 'verify'] }, texts, VERSION).includes(texts.fixConfirmStale))
  assert.ok(!fixConfirmText({ kind: 'clean-install', steps: ['cleanup', 'install', 'verify'] }, texts, VERSION).includes(texts.fixEndsStream))
})

// ---- what cannot be fixed by software gets steps ----
test('each failure that software cannot finish maps to a guide, and every guide has its steps in every language', async () => {
  const cases = {
    'USBIP-SETUP-016: Windows driver policy blocked': 'policy',
    'USBIP-SETUP-007: transport installation failed: USBIP-SETUP-016: blocked': 'policy',
    'USBIP-SETUP-017: Smart App Control blocked': 'sac',
    'USBIP-SETUP-011: USB root hub check failed and was rolled back': 'roothub',
    'USBIP-SETUP-012: rollback incomplete': 'roothub-broken',
    'USBIP-REPLACE-003: root hubs not recovered': 'roothub-broken',
    'USBIP-SETUP-013: Windows boots from a USB disk': 'usb-boot',
    'USBIP-SETUP-014: guard could not run': 'guard',
    'USBIP-CLEAN-007: access denied': 'access',
    'USBIP-SETUP-008: externally managed': 'external',
    'USBIP-REPLACE-001: cannot be replaced from here': 'external',
    'USBIP-SETUP-010: installer failed': 'generic',
    'USBIP-REPLACE-004: failed': 'generic',
    'USBIP-CLEAN-003: timed out': 'generic',
    'USBIP-SETUP-006: stream active': '',
    'USBIP-SETUP-018: busy': '',
    'USBIP-CLEAN-008: in use': '',
    'USBIP-UAC-001: canceled': '',
  }
  for (const [error, guide] of Object.entries(cases)) assert.equal(guideFor(error), guide, error)
  for (const locale of DESKTOP_LOCALES) {
    const table = locale === 'en' ? en : locale === 'zh' ? zh : (await import(`../desktop/i18n/${locale}.js`))[locale]
    for (const id of FIX_GUIDE_IDS) {
      const text = fixGuideText(id, table.deviceHub.usb, VERSION)
      assert.ok(text.split('\n').length >= 3, `${locale}/${id}: a sentence and its steps`)
      assert.ok(!text.includes('{version}'), `${locale}/${id}: nothing left unfilled`)
    }
  }
  const failed = describeFixResult({ ...initialFix(), outcome: 'failed', failedStep: 'install', error: 'USBIP-SETUP-007: transport installation failed: USBIP-SETUP-016: blocked' }, usb.zh, friendly(usb.zh), VERSION)
  assert.deepEqual([failed.action, failed.guide], ['guide', 'policy'])
  assert.ok(fixGuideText('policy', usb.zh, VERSION).includes('内存完整性'))
  assert.ok(fixGuideText('external', usb.zh, VERSION).includes(VERSION))
})

// ---- a repair that waits for a restart continues afterwards ----
const memory = () => {
  const map = new Map()
  return { getItem: (k) => map.get(k) ?? null, setItem: (k, v) => map.set(k, String(v)), removeItem: (k) => map.delete(k) }
}

test('a restart keeps the plan; the next run of the panel loads it once, and an old or same-run record is not used', () => {
  const store = memory()
  saveFixResume(store, { id: 'mismatch-hash-abi', steps: ['end-stream', 'cleanup', 'install', 'verify'] }, 1000)
  assert.equal(loadFixResume(store, { bootMs: 500, now: 2000 }), null, 'the same Windows session (it started before the plan was kept) does not continue it')
  assert.equal(loadFixResume(store, { bootMs: 1000, now: 2000 }), null, 'not earlier than the write either')
  assert.equal(loadFixResume(store, { bootMs: 0, now: 2000 }), null, 'without a boot time nothing is claimed')
  const kept = loadFixResume(store, { bootMs: 1500, now: 2000 })
  assert.deepEqual(kept, { id: 'mismatch-hash-abi', steps: ['cleanup', 'install', 'verify'], ports: [], written: 1000 }, 'a record that is not claimable yet stays, and is claimed once Windows has started after it')
  assert.equal(loadFixResume(store, { bootMs: 1500, now: 1000 + 25 * 60 * 60 * 1000 }), null, 'a day later it is gone')
  assert.equal(loadFixResume(store, { bootMs: 1500, now: 2000 }), null, 'and was dropped')
  clearFixResume(store)
  assert.equal(loadFixResume(store, { bootMs: 1500, now: 2000 }), null)
  assert.equal(loadFixResume({ getItem() { throw new Error('private mode') } }, { bootMs: 2, now: 3 }), null)
  saveFixResume(store, { id: 'external-incompatible-replaceable', steps: ['detach', 'replace', 'verify'], ports: [2, 1] }, 1000)
  assert.deepEqual(loadFixResume(store, { bootMs: 1500, now: 2000 }).ports, [2, 1], 'the devices that were confirmed are kept with the steps')
})

test('after the restart the rest of the plan runs without a question; anything beyond it, a new stream or a new device included, asks again', async () => {
  const resume = { id: 'mismatch-hash-abi', steps: ['cleanup', 'install', 'verify'], ports: [] }
  // Windows finished the cleanup: only the install is left
  const left = world({ status: { ...base, installed: false, ready: false }, sessions: [] })
  const done = await runUsbipFix(left.deps, undefined, { resume })
  assert.equal(done.outcome, 'done')
  assert.equal(done.resumed, true)
  assert.deepEqual(left.calls.filter((c) => c !== 'sessions'), ['refresh', 'install', 'verify'], 'no question was asked')
  // everything is already fine: only the check runs
  const fine = world({ status: base })
  assert.equal((await runUsbipFix(fine.deps, undefined, { resume })).outcome, 'done')
  assert.deepEqual(fine.calls, ['refresh', 'verify'])
  // a stream that was not there when the plan was confirmed is not ended on that old answer: the question comes again, and the stream is ended only after a yes
  const stream = world({ status: { ...base, installed: false, ready: false }, sessions: [{ session_id: 1 }], streamEndsAfter: 3 })
  assert.equal((await runUsbipFix(stream.deps, undefined, { resume })).outcome, 'done')
  assert.equal(world.questions.at(-1).streaming, true)
  assert.ok(stream.calls.indexOf('confirm') < stream.calls.indexOf('endStream'), 'asked before the stream was ended')
  const refused = world({ status: { ...base, installed: false, ready: false }, sessions: [{ session_id: 1 }], confirm: false })
  assert.equal((await runUsbipFix(refused.deps, undefined, { resume })).outcome, 'cancelled')
  assert.equal(refused.calls.includes('endStream'), false, 'a no leaves the stream alone')
  // a replacement the person did not confirm asks; a no changes nothing
  const other = world({ status: { ...base, ready: false, externally_managed: true, replaceable: true, version_valid: false }, sessions: [], confirm: false })
  assert.equal((await runUsbipFix(other.deps, undefined, { resume })).outcome, 'cancelled')
  assert.equal(other.calls.includes('replace'), false)
  assert.ok(other.calls.includes('confirm'))
  // a device that was not among the confirmed ones asks again
  const device = world({ status: { ...mismatch, attached_devices: [{ port: 3 }] }, sessions: [], confirm: false })
  assert.equal((await runUsbipFix(device.deps, undefined, { resume: { ...resume, steps: ['detach', 'cleanup', 'install', 'verify'], ports: [1] } })).outcome, 'cancelled')
  assert.ok(device.calls.includes('confirm'))
  const same = world({ status: { ...mismatch, attached_devices: [{ port: 1 }] }, sessions: [] })
  assert.equal((await runUsbipFix(same.deps, undefined, { resume: { ...resume, steps: ['detach', 'cleanup', 'install', 'verify'], ports: [1] } })).outcome, 'done')
  assert.equal(same.calls.includes('confirm'), false)
  // a second restart keeps the plan again
  const again = world({ status: { ...base, installed: false, ready: false }, sessions: [], install: { ok: true, data: { ready: false, install_pending_restart: true } } })
  const kept = []
  again.deps.onReboot = ({ plan }) => kept.push(plan.steps)
  assert.equal((await runUsbipFix(again.deps, undefined, { resume })).outcome, 'reboot-install')
  assert.deepEqual(kept, [['install', 'verify']])
})

// ---- phases never overlap ----
/** A system that changes while the question is open: `statuses` and `streams` are what each look gives, one after the other. */
function changing({ statuses, streams, confirm = true }) {
  const { deps, calls } = world({ sessions: [] })
  const looks = [...statuses]
  const streamLooks = [...streams]
  let ended = false
  deps.refresh = async () => { calls.push('refresh'); return looks.length > 1 ? looks.shift() : looks[0] }
  deps.endStream = async () => { calls.push('endStream'); ended = true; return { ok: true } }
  deps.sessions = async () => { calls.push('sessions'); return ended ? [] : streamLooks.length > 1 ? streamLooks.shift() : streamLooks[0] }
  deps.confirm = async (question) => { calls.push('confirm'); world.questions.push(question); return typeof confirm === 'function' ? confirm(question) : confirm }
  return { deps, calls }
}

test('a stream that starts while the question is open is not changed under: the question comes again, naming the stream', async () => {
  const { deps, calls } = changing({ statuses: [mismatch], streams: [[], [{ session_id: 1 }]] })
  const asked = []
  const originalConfirm = deps.confirm
  deps.confirm = async (question) => { asked.push(question.plan.steps); return originalConfirm(question) }
  const state = await runUsbipFix(deps)
  assert.equal(state.outcome, 'done')
  assert.deepEqual(asked, [['cleanup', 'install', 'verify'], ['end-stream', 'cleanup', 'install', 'verify']], 'the second question names the stream that will end')
  assert.ok(calls.indexOf('endStream') > calls.lastIndexOf('confirm'), 'the stream is ended only after the answer to the second question')
})

test('a device attached while the question is open comes into the question before it is released', async () => {
  const { deps } = changing({ statuses: [mismatch, { ...mismatch, attached_devices: [{ port: 4 }] }], streams: [[]] })
  const asked = []
  const originalConfirm = deps.confirm
  deps.confirm = async (question) => { asked.push([question.plan.steps, question.plan.ports]); return originalConfirm(question) }
  const released = []
  deps.detach = async (ports) => { released.push(ports); return { ok: true } }
  assert.equal((await runUsbipFix(deps)).outcome, 'done')
  assert.deepEqual(asked, [[['cleanup', 'install', 'verify'], []], [['detach', 'cleanup', 'install', 'verify'], [4]]])
  assert.deepEqual(released, [[4]], 'only the devices that were confirmed are released')
})

test('a no to the second question changes nothing, and a system that keeps changing ends the questions', async () => {
  const declined = changing({ statuses: [mismatch], streams: [[], [{ session_id: 1 }]], confirm: (question) => !question.streaming })
  assert.equal((await runUsbipFix(declined.deps)).outcome, 'cancelled')
  for (const step of ['endStream', 'cleanup', 'install']) assert.equal(declined.calls.includes(step), false)
  let flip = 0
  const unstable = changing({ statuses: [mismatch], streams: [[]] })
  unstable.deps.sessions = async () => { unstable.calls.push('sessions'); flip += 1; return flip % 2 ? [] : [{ session_id: flip }, { session_id: flip + 1 }].slice(0, 1 + (flip % 3)) }
  const state = await runUsbipFix(unstable.deps)
  assert.deepEqual([state.outcome, state.failedStep], ['failed', 'check'])
  assert.equal(unstable.calls.includes('cleanup'), false)
})

test('the system turning healthy while the question is open ends the repair with nothing done', async () => {
  const { deps, calls } = changing({ statuses: [mismatch, base], streams: [[]] })
  const state = await runUsbipFix(deps)
  assert.equal(state.outcome, 'healthy')
  assert.equal(calls.includes('cleanup'), false)
})

// ---- a restart that was already waiting is kept too, and the order of an install result ----
test('a restart that was already waiting when the repair began is kept with the steps it finishes', async () => {
  for (const [status, id, steps] of [
    [{ ...mismatch, cleanup_pending_restart: true }, 'pending-restart-cleanup', ['cleanup', 'verify']],
    [{ ...base, installed: false, ready: false, install_pending_restart: true }, 'pending-restart-install', ['install', 'verify']],
    [{ ...base, reboot_recommended: true }, 'reboot-recommended', ['verify']],
  ]) {
    const { deps } = world({ status })
    const kept = []
    deps.onReboot = ({ plan, kind }) => kept.push([plan.id, plan.steps, kind])
    const state = await runUsbipFix(deps)
    assert.match(state.outcome, /^reboot-/)
    assert.equal(kept.length, 1, id)
    assert.deepEqual(kept[0].slice(0, 2), [id, steps], id)
  }
})

test('an install that answers ready and still asks for a restart is a restart; one that is not ready and says nothing is an ordinary failure with its own steps', async () => {
  const both = await runUsbipFix(world({ sessions: [], install: { ok: true, data: { ready: true, reboot_recommended: true } } }).deps)
  assert.equal(both.outcome, 'reboot-install')
  const silent = await runUsbipFix(world({ sessions: [], install: { ok: true, data: { ready: false } } }).deps)
  assert.deepEqual([silent.outcome, silent.failedStep], ['failed', 'install'])
  assert.equal(primaryCode(silent.error), 'USBIP-SETUP-010')
  const result = describeFixResult(silent, usb.zh, friendly(usb.zh), VERSION)
  assert.equal(result.guide, 'generic', 'an ordinary install failure is not the steps for removing an installation of the person')
  assert.equal(guideFor('USBIP-SETUP-008: externally managed incompatible USB/IP is installed (exit code 106)'), 'external', 'the real external error keeps its steps')
})

// ---- a failure is reported once, and keeps what was done before it ----
test('a failure the step already reported is not reported again; one the repair made itself is', async () => {
  const own = await runUsbipFix(world({ sessions: [], cleanup: { ok: false, error: 'USBIP-CLEAN-008: busy', reported: true } }).deps)
  assert.equal(own.reported, true)
  const synthetic = await runUsbipFix(world({ sessions: [], cleanup: { ok: true, data: { vhci_residual: true } } }).deps)
  assert.equal(synthetic.reported, false)
  const check = await runUsbipFix(world({ sessions: [], verify: { ok: false, error: 'USBIP-VERIFY-001: not ready' } }).deps)
  assert.equal(check.reported, false)
})

test('devices released before a release failed stay in the diagnostics of the failure', async () => {
  const { deps } = world({ status: { ...mismatch, attached_devices: [{ port: 1 }, { port: 2 }] }, sessions: [], detach: { ok: false, error: 'USBIP-UAC-001: canceled', raw: 'port 1: released' } })
  const state = await runUsbipFix(deps)
  assert.equal(state.failedStep, 'detach')
  assert.match(usbipFixRawText(state), /detach: failed\nport 1: released\nUSBIP-UAC-001: canceled/)
})

test('the document of the inventory is the one the inventory gives', async () => {
  const { readFileSync } = await import('node:fs')
  const { DOC_PATH, renderInventoryDoc } = await import('../../../scripts/usbip-fix-inventory-doc.mjs')
  assert.equal(readFileSync(DOC_PATH, 'utf8'), renderInventoryDoc(), 'run node scripts/usbip-fix-inventory-doc.mjs')
})
