import assert from 'node:assert/strict'
import test from 'node:test'
import { createUpdateDownload } from './updateDownload.js'

const deferred = () => {
  let resolve
  let reject
  const promise = new Promise((next, fail) => { resolve = next; reject = fail })
  return { promise, resolve, reject }
}

const harness = ({ status = { active: false } } = {}) => {
  const api = {
    events: [],
    listener: null,
    status,
    download: deferred(),
    async invoke(command, args) {
      api.events.push({ command, args })
      if (command === 'download_update_status') return api.status
      return api.download.promise
    },
    async listen(name, callback) {
      api.listener = { name, callback }
      return () => { api.listener = null }
    },
    progress(payload) { api.listener?.callback({ payload }) },
  }
  return { api, download: createUpdateDownload({ invoke: api.invoke, listen: api.listen }) }
}

const tick = () => new Promise((resolve) => setImmediate(resolve))

test('progress stays in the store when the sheet is not there', async () => {
  const { api, download } = harness()
  const task = download.start({ url: 'https://cnb.example/a.exe', filename: 'a.exe' })
  await tick()
  api.progress({ progress: 42, downloaded: 420, total: 1000, phase: 'downloading', source: 'CNB' })
  assert.equal(download.state.active, true)
  assert.equal(download.percent.value, 42)
  assert.equal(download.state.source, 'CNB')

  api.download.resolve({ success: true, file_path: 'C:/tmp/a.exe' })
  assert.equal((await task).ok, true)
  assert.equal(download.state.active, false)
  assert.equal(download.state.path, 'C:/tmp/a.exe')
  assert.equal(download.ready.value, true)
  assert.equal(download.visible.value, true)
})

test('a second start shares the transfer that is already running', async () => {
  const { api, download } = harness()
  const first = download.start({ url: 'https://cnb.example/a.exe', filename: 'a.exe' })
  await tick()
  const second = await download.start({ url: 'https://cnb.example/a.exe', filename: 'a.exe' })
  assert.equal(second.busy, true)
  const downloads = api.events.filter((event) => event.command === 'download_update')
  assert.equal(downloads.length, 1)
  api.download.resolve({ success: true, file_path: 'C:/tmp/a.exe' })
  await first
})

test('a transfer started before a webview reload is read back from the native task', async () => {
  const { download } = harness({
    status: {
      active: true,
      phase: 'downloading',
      progress: 17,
      downloaded: 170,
      total: 1000,
      source: 'CNB',
      filename: 'a.exe',
      error: null,
    },
  })
  await download.refresh()
  assert.equal(download.state.active, true)
  assert.equal(download.percent.value, 17)
  assert.equal(download.state.filename, 'a.exe')
})

test('a failed transfer keeps its error code for the localized message', async () => {
  const { api, download } = harness()
  const task = download.start({ url: 'https://cnb.example/a.exe', filename: 'a.exe' })
  await tick()
  api.download.reject({ code: 'sources_exhausted' })
  const result = await task
  assert.equal(result.ok, false)
  assert.equal(download.state.errorCode, 'sources_exhausted')
})
