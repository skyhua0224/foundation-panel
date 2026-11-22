import test from 'node:test'
import assert from 'node:assert/strict'
import { ref } from 'vue'
import { useDeviceComponentOperation } from './useDeviceComponentOperation.js'
import { useVbCableComponent } from './useVbCableComponent.js'

function fixture(initial, overrides = {}, confirmation = async () => {}) {
  const calls = []
  const text = ref({ downloaded: 'downloaded', installed: 'installed', enabled: 'enabled', uninstalled: 'uninstalled (restart may be needed)', restartSuggested: 'restart may be needed',
    states: { pending: 'driver remains', installed: 'driver still installed' }, notChecked: 'not checked',
    checkFailed: 'check failed', downloadFailed: 'download failed', installFailed: 'install failed', enableFailed: 'enable failed', uninstallFailed: 'uninstall failed', unsupportedArchitecture: 'unsupported architecture',
    operationTimeout: 'operation timed out', operationBusy: 'operation busy', deviceBusy: 'device busy' })
  const api = {
    check: async () => { calls.push('check'); return { success: true, data: initial } },
    download: async () => { calls.push('download'); return { success: true } },
    install: async () => { calls.push('install'); return { success: true, data: { status: { state: 'installed', archive_ready: true }, restart_suggested: false } } },
    enable: async () => { calls.push('enable'); return { success: true, data: { status: { state: 'installed', archive_ready: false }, restart_suggested: false } } },
    uninstall: async () => { calls.push('uninstall'); return { success: true, data: { status: { state: 'missing', archive_ready: true }, restart_suggested: false } } },
    ...overrides,
  }
  const manager = useVbCableComponent({ api, text, shared: useDeviceComponentOperation(), confirm: async () => { calls.push('confirm'); await confirmation() }, invalidate: () => calls.push('invalidate') })
  return { manager, calls }
}

test('checking and downloading do not install; installation is an explicit later action', async () => {
  const { manager, calls } = fixture({ state: 'missing', archive_ready: false })
  await manager.check()
  assert.equal(manager.canInstall.value, false)
  await manager.download()
  assert.deepEqual(calls, ['check', 'download'])
  assert.equal(manager.canInstall.value, true)
  await manager.changeDriver('install')
  assert.deepEqual(calls, ['check', 'download', 'confirm', 'invalidate', 'install', 'invalidate'])
  assert.equal(manager.status.value.state, 'installed')
})

test('a missing device cannot be uninstalled', async () => {
  const { manager, calls } = fixture({ state: 'missing', archive_ready: true })
  await manager.check()
  assert.equal(manager.hasRemovableDriver.value, false)
  assert.equal(manager.canUninstall.value, false)
  await manager.changeDriver('uninstall')
  assert.deepEqual(calls, ['check'])
})

test('a disabled device can be enabled without downloading or reinstalling', async () => {
  const { manager, calls } = fixture({ state: 'disabled', archive_ready: false })
  await manager.check()
  assert.equal(manager.canEnable.value, true)
  assert.equal(manager.canInstall.value, false)
  await manager.changeDriver('enable')
  assert.deepEqual(calls, ['check', 'confirm', 'invalidate', 'enable', 'invalidate'])
  assert.equal(manager.status.value.state, 'installed')
  assert.equal(manager.notice.value, 'enabled')
})

test('checking or cancelling repair never enables a disabled device', async () => {
  const { manager, calls } = fixture({ state: 'disabled', archive_ready: false }, {}, async () => { throw new Error('cancelled') })
  await manager.check()
  assert.deepEqual(calls, ['check'])
  assert.equal(manager.canEnable.value, true)
  await manager.changeDriver('enable')
  assert.deepEqual(calls, ['check', 'confirm'])
  assert.equal(manager.status.value.state, 'disabled')
  assert.equal(manager.canEnable.value, true)
  assert.equal(manager.phase.value, '')
})

test('cancelling uninstall leaves the driver unchanged', async () => {
  const { manager, calls } = fixture({ state: 'installed', archive_ready: false }, {}, async () => { throw new Error('cancelled') })
  await manager.check()
  await manager.changeDriver('uninstall')
  assert.deepEqual(calls, ['check', 'confirm'])
  assert.equal(manager.status.value.state, 'installed')
  assert.equal(manager.canUninstall.value, true)
  assert.equal(manager.phase.value, '')
})

test('driver timeout and both kinds of occupation have separate messages', async () => {
  for (const [reason, expected] of [
    ['VBCABLE-OP-TIMEOUT: still running', 'operation timed out'],
    ['VBCABLE-OP-BUSY: another operation', 'operation busy'],
    ['VB-CABLE: failed: VBCABLE-DEVICE-BUSY: in use', 'device busy'],
  ]) {
    const { manager } = fixture({ state: 'installed', archive_ready: false }, {
      uninstall: async () => ({ success: false, message: reason }),
    })
    await manager.check()
    await manager.changeDriver('uninstall')
    assert.equal(manager.notice.value, expected)
  }
})

test('a remaining driver can be uninstalled without allowing installation over it', async () => {
  const { manager, calls } = fixture({ state: 'pending', archive_ready: true })
  await manager.check()
  await manager.changeDriver('install')
  assert.equal(manager.canInstall.value, false)
  assert.equal(manager.hasRemovableDriver.value, true)
  assert.equal(manager.canUninstall.value, true)
  await manager.changeDriver('uninstall')
  assert.deepEqual(calls, ['check', 'confirm', 'invalidate', 'uninstall', 'invalidate'])
  assert.equal(manager.status.value.state, 'missing')
  assert.match(manager.notice.value, /restart may be needed/)
})

test('3010 reports the remaining driver and does not block another cleanup attempt', async () => {
  const { manager, calls } = fixture({ state: 'pending', archive_ready: true }, {
    uninstall: async () => { calls.push('uninstall'); return { success: true, data: { status: { state: 'pending', archive_ready: true }, restart_suggested: true } } },
  })
  await manager.check()
  await manager.changeDriver('uninstall')
  assert.equal(manager.notice.value, 'driver remains restart may be needed')
  await manager.check()
  assert.equal(manager.canUninstall.value, true)
  await manager.changeDriver('uninstall')
  assert.equal(calls.filter(call => call === 'uninstall').length, 2)
})

test('a failed download does not make installation available', async () => {
  const { manager } = fixture({ state: 'missing', archive_ready: false }, { download: async () => ({ success: false, message: 'offline' }) })
  await manager.check()
  await manager.download()
  assert.equal(manager.canInstall.value, false)
  assert.match(manager.notice.value, /download failed.*offline/)
})

test('an unsupported installation architecture does not turn an existing device into a missing one', async () => {
  const { manager } = fixture({ state: 'missing', archive_ready: true }, {
    install: async () => ({ success: false, message: 'VBCABLE-ARCH-UNSUPPORTED: x64 required' }),
  })
  await manager.check()
  await manager.changeDriver('install')
  assert.equal(manager.notice.value, 'unsupported architecture')
  assert.equal(manager.status.value.state, 'missing')
  const existing = fixture({ state: 'installed', archive_ready: false })
  await existing.manager.check()
  assert.equal(existing.manager.status.value.state, 'installed')
  assert.equal(existing.manager.canUninstall.value, true)
})
