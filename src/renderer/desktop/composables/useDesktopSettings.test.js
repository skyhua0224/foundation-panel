import assert from 'node:assert/strict'
import test from 'node:test'

const store = new Map()
globalThis.localStorage = {
  getItem: (k) => store.get(k) ?? null,
  setItem: (k, v) => store.set(k, String(v)),
  removeItem: (k) => store.delete(k),
}
globalThis.CustomEvent ??= class CustomEvent {
  constructor(type, init = {}) {
    this.type = type
    this.detail = init.detail
  }
}

let shouldFail = false
const tauriInternals = {
  transformCallback: () => 1,
  invoke: async (command, args) => {
    if (command === 'get_desktop_settings') {
      return { settings: { autoStart: true, startMinimized: true, autoStartSunshine: true }, status: { autoStartRegistered: true } }
    }
    if (command === 'save_desktop_settings') {
      if (shouldFail) throw new Error('Access denied')
      return { settings: args.settings, status: { autoStartRegistered: args.settings.autoStart } }
    }
    return null
  },
}
globalThis.isTauri = true
globalThis.__TAURI_INTERNALS__ = tauriInternals
globalThis.window = {
  addEventListener() {},
  removeEventListener() {},
  dispatchEvent() {},
  __TAURI_INTERNALS__: tauriInternals,
}

const { DESKTOP_SETTINGS_KEY, desktopSettings, loadDesktopSettings, saveDesktopSettings } = await import('./useDesktopSettings.js')

test('saveDesktopSettings keeps previous state and storage intact when backend rejects', async () => {
  await loadDesktopSettings()
  assert.equal(desktopSettings.value.autoStart, true)
  const storedBefore = store.get(DESKTOP_SETTINGS_KEY)

  shouldFail = true
  await assert.rejects(
    () => saveDesktopSettings({ ...desktopSettings.value, autoStart: false }),
    /Access denied/,
  )
  assert.equal(desktopSettings.value.autoStart, true)
  assert.equal(store.get(DESKTOP_SETTINGS_KEY), storedBefore)

  shouldFail = false
  await saveDesktopSettings({ ...desktopSettings.value, autoStart: false })
  assert.equal(desktopSettings.value.autoStart, false)
  assert.equal(JSON.parse(store.get(DESKTOP_SETTINGS_KEY)).autoStart, false)
})
