// The developer-mode sign-in of the panel: 「打开 github.com/login/device」 did nothing because the panel gave foundation-ui's sheets no openUrl.
import assert from 'node:assert/strict'
import test from 'node:test'
import { readFileSync } from 'node:fs'

const calls = []
let answer = () => true
globalThis.window = { __TAURI_INTERNALS__: { invoke: async (cmd, args) => { calls.push([cmd, args]); return answer() }, transformCallback: () => 1 } }
const { openExternalUrl } = await import('../tauri-adapter.js')

test('the panel registers openUrl for the report and sign-in sheets, and it is the native opener', () => {
  const src = readFileSync(new URL('./developerReport.js', import.meta.url), 'utf8')
  assert.match(src, /registerDevReport\(\{[\s\S]*openUrl: \(url\) => openExternalUrl\(url\)/)
  assert.match(src, /import \{[^}]*openExternalUrl[^}]*\} from '\.\.\/tauri-adapter\.js'/)
})

test('the panel tells the feedback sheet that its host takes a screenshot, and the capture it gives is the native window capture', () => {
  const src = readFileSync(new URL('./developerReport.js', import.meta.url), 'utf8')
  assert.match(src, /registerDevReport\(\{[\s\S]*shots: true/)
  assert.match(src, /capture: async \(\) => bytesOfCapture\(await invoke\('dev_capture_window'\)\)/)
})

test('the capture arrives as the PNG bytes of a binary answer; text in base64 still works', async () => {
  const { bytesOfCapture } = await import('./developerTools.js')
  const png = Uint8Array.from([137, 80, 78, 71, 13, 10, 26, 10])
  assert.deepEqual([...bytesOfCapture(png.buffer.slice(0))], [...png])
  assert.deepEqual([...bytesOfCapture(png)], [...png])
  assert.deepEqual([...bytesOfCapture(Buffer.from(png).toString('base64'))], [...png])
  assert.equal(bytesOfCapture(null).length, 0)
})

test('the device page goes to the native opener and its answer is the sheet\'s: true opened, false could not', async () => {
  assert.equal(await openExternalUrl('https://github.com/login/device'), true)
  assert.deepEqual(calls.at(-1), ['open_external_url', { url: 'https://github.com/login/device' }])
  answer = () => { throw new Error('ShellExecuteW code 5') }
  assert.equal(await openExternalUrl('https://github.com/login/device'), false)
  answer = () => true
  assert.equal(await openExternalUrl('file:///C:/Windows/notepad.exe'), false, 'only web and store addresses')
  assert.equal(calls.filter(([c]) => c === 'open_external_url').length, 2, 'a refused address never reaches the native side')
})
