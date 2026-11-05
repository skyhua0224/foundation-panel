import assert from 'node:assert/strict'
import fs from 'node:fs'
import vm from 'node:vm'
import test from 'node:test'
import { en } from '../desktop/i18n/en.js'
import { zh } from '../desktop/i18n/zh.js'
import { formatMessage } from '../shared/format-message.js'

// Execute the composable with inert UI/Tauri boundaries: never download or launch a tool.
const source = fs.readFileSync(new URL('./useTools.js', import.meta.url), 'utf8')
  .replace(/^import .*\r?\n/gm, '')
  .replace('export function useTools()', 'function useTools()')
  .replace(/import\('(@tauri-apps\/api\/[^']+)'\)/g, 'loadTauri("$1")')

function harness(messages = en, options = {}) {
  const calls = { dialogs: [], messages: [], urls: [], busy: [], downloads: [], launches: 0, statusCalls: 0, noticeCloses: 0, loadingCloses: 0 }
  let progressListener
  let unlistened = false
  let api = null
  const t = { value: messages }
  const ElMessage = (options) => { calls.messages.push(options.message); return { close: () => { calls.loadingCloses += 1 } } }
  for (const type of ['info', 'success', 'error', 'warning']) {
    ElMessage[type] = (message) => calls.messages.push(message)
  }
  const ElMessageBox = async (config) => {
    calls.dialogs.push(config)
    if (options.choice === 'cancel' || options.choice === 'close') throw options.choice
    return 'confirm'
  }
  ElMessageBox.confirm = async (message, title, config) => {
    calls.dialogs.push({ message, title, ...config })
    return 'confirm'
  }
  ElMessageBox.alert = async (message, title, config) => {
    calls.dialogs.push({ message, title, ...config })
    return 'confirm'
  }
  api = vm.runInNewContext(`${source}\nuseTools()`, {
    ref: (value) => ({ value }), useI18n: () => ({ t }), formatMessage,
    ElMessage, ElMessageBox, ElNotification: (config) => { calls.dialogs.push(config); return { close: () => { calls.noticeCloses += 1 } } },
    openExternalUrl: async (url) => calls.urls.push(url), tools: {},
    console: { error() {}, warn() {}, log() {} },
    controllerMeta: {
      getStatus: async () => {
        calls.statusCalls += 1
        if (options.statusError) throw options.statusError
        return { installed: options.installed || false, version: '1.0' }
      },
      checkRelease: async () => {
        if (options.releaseError) throw options.releaseError
        return { download_url: 'https://example.com/tool.exe', version: '2.0', download_size: 1048576 }
      },
      download: async (...args) => {
        calls.downloads.push(args)
        calls.busy.push(api?.controllerBusyText?.value)
        if (options.duringDownload) options.duringDownload(t)
        progressListener({ payload: { progress: 50, downloaded: 524288 } })
        calls.busy.push(api?.controllerBusyText?.value)
        if (options.downloadError) throw options.downloadError
      },
      launch: async () => { calls.launches++; if (options.launchError) throw options.launchError },
    },
    loadTauri: async () => ({
      listen: async (_, listener) => { progressListener = listener; return () => { unlistened = true } },
      invoke: async (command) => {
        if (command === 'is_running_as_admin') return true
        if (command === 'cleanup_unused_covers' && options.cleanupError) throw options.cleanupError
        return { success: true, deleted_count: options.deletedCount ?? 2, freed_space: 2048, message: '成功删除无用文件' }
      },
    }),
  })
  return { api, calls, t, unlistened: () => unlistened }
}

test('English Gamepad Test prompt, download progress and launch use translated text', async () => {
  const { api, calls, unlistened } = harness()
  await api.openGamepadTest()
  assert.equal(calls.dialogs[0].title, 'Gamepad Test Tool')
  assert.equal(calls.dialogs[0].confirmButtonText, 'Download and Launch')
  assert.equal(calls.dialogs[0].cancelButtonText, 'Open Web Version')
  assert.equal(calls.dialogs[0].distinguishCancelAndClose, true)
  assert.match(calls.busy.join('\n'), /0\.5\/1\.0 MB \(50%\)/)
  assert.equal(calls.downloads.length, 1)
  assert.equal(calls.launches, 1)
  assert.equal(unlistened(), true)
  assert.doesNotMatch(JSON.stringify(calls), /\p{Script=Han}/u)
})

test('an update notice stands with its action and closes when the update lands; a second run does not start while it downloads', async () => {
  let again
  const h = harness(en, { installed: true, duringDownload: () => { again = h.api.openGamepadTest() } })
  await h.api.openGamepadTest()
  await new Promise((resolve) => setTimeout(resolve, 10))
  const notice = h.calls.dialogs.find((dialog) => dialog.actionText)
  assert.equal(notice.actionText, en.controllerMeta.updateAction)
  assert.equal(notice.duration, 0)
  assert.equal(notice.standing, 'controllermeta-update')
  assert.equal(notice.revision, '2.0')
  await notice.onClick()
  await again
  assert.equal(h.calls.downloads.length, 1)
  assert.equal(h.calls.launches, 2)
  assert.equal(h.calls.statusCalls, 1)
  assert.equal(h.calls.noticeCloses, 1)
  assert.equal(h.calls.messages.includes(en.controllerMeta.preparing), false)
  await h.api.openGamepadTest()
  assert.equal(h.calls.statusCalls, 2)
})

test('web choice opens only the fallback; closing the prompt performs no action', async () => {
  for (const choice of ['cancel', 'close']) {
    const { api, calls } = harness(en, { choice })
    await api.openGamepadTest()
    assert.equal(calls.downloads.length, 0)
    assert.equal(calls.launches, 0)
    assert.equal(calls.urls.length, choice === 'cancel' ? 1 : 0)
    if (choice === 'cancel') assert.equal(calls.urls[0], 'https://hardwaretester.com/gamepad')
  }
})

test('the busy state is taken before the first await and given back after a throw', async () => {
  const h = harness(en, { statusError: new Error('status query failed') })
  const run = h.api.openGamepadTest()
  assert.equal(h.api.controllerBusy.value, true)
  assert.equal(h.api.controllerBusyText.value, '')
  await run
  assert.equal(h.api.controllerBusy.value, false)
  assert.equal(h.api.controllerBusyText.value, '')
  assert.equal(h.calls.messages.at(-1), en.controllerMeta.unavailable)
  assert.deepEqual(h.calls.urls, ['https://hardwaretester.com/gamepad'])
})

test('the blocking layer carries the live progress and goes away on success and failure', async () => {
  const done = harness(en)
  await done.api.openGamepadTest()
  assert.match(done.calls.busy.at(-1), /0\.5\/1\.0 MB \(50%\)/)
  assert.equal(done.api.controllerBusyText.value, '')
  assert.equal(done.api.controllerBusy.value, false)
  const failed = harness(en, { downloadError: new Error('write failed') })
  await failed.api.openGamepadTest()
  assert.equal(failed.api.controllerBusyText.value, '')
  assert.equal(failed.api.controllerBusy.value, false)
  assert.equal(failed.calls.messages.at(-1), en.controllerMeta.downloadFailed)
})

test('Chinese prompt remains available and async progress follows the current locale', async () => {
  const { api, calls } = harness(zh, { duringDownload: (t) => { t.value = en } })
  await api.openGamepadTest()
  assert.equal(calls.dialogs[0].title, zh.controllerMeta.title)
  assert.match(calls.busy.at(-1), /Downloading/)
  assert.match(calls.messages.at(-1), /installed/)
})

test('backend failures do not leak Chinese into English notifications', async () => {
  for (const options of [
    { releaseError: '查询 GitHub Release 失败' },
    { downloadError: '无法覆盖已有版本，请先关闭 ControllerMeta 后重试: os error 5' },
    { installed: true, launchError: 'ControllerMeta 未安装' },
  ]) {
    const { api, calls } = harness(en, options)
    await api.openGamepadTest()
    assert.doesNotMatch(JSON.stringify(calls), /\p{Script=Han}/u)
    if (options.downloadError) assert.equal(calls.messages.at(-1), en.controllerMeta.closeBeforeUpdate)
    if (options.launchError) assert.equal(calls.messages.at(-1), en.controllerMeta.notInstalled)
  }
})

test('cleanup formats counters locally instead of displaying backend Chinese text', async () => {
  for (const deletedCount of [0, 2]) {
    const { api, calls } = harness(en, { deletedCount })
    await api.cleanupCovers()
    assert.doesNotMatch(JSON.stringify(calls), /\p{Script=Han}/u)
    if (deletedCount) assert.match(calls.dialogs.at(-1).message, /Deleted files: 2\nFreed space: 2\.00 KB/)
    else assert.equal(calls.messages.at(-1), en.toolActions.cleanupEmpty)
  }
})

test('the cleaning message ends on every path', async () => {
  const h = harness(en, { cleanupError: new Error('disk full') })
  await h.api.cleanupCovers()
  assert.equal(h.calls.loadingCloses, 1)
  assert.equal(h.calls.messages.at(-1), en.toolActions.cleanupFailed)
})

test('new translations have matching keys and placeholder names in both supported locales', () => {
  for (const section of ['toolActions', 'controllerMeta', 'coverPicker', 'windowActions', 'routeTitles']) {
    assert.deepEqual(Object.keys(en[section]).sort(), Object.keys(zh[section]).sort())
    for (const key of Object.keys(en[section])) {
      assert.deepEqual(en[section][key].match(/\{\w+\}/g), zh[section][key].match(/\{\w+\}/g))
    }
  }
})

test('message formatting preserves dollar signs and braces in user-provided values', () => {
  assert.equal(formatMessage('{name}: {count}', { name: '$& {count}', count: 2 }), '$& {count}: 2')
})
