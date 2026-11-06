import assert from 'node:assert/strict'
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import test from 'node:test'
import { commitOsk, editInputWithOsk } from './composables/useOsk.js'
import { en } from './i18n/en.js'
import { zh } from './i18n/zh.js'

// review rv-md-desk2 (2026-11-06): the six findings, each kept as a test on the merged head
const root = fileURLToPath(new URL('./', import.meta.url))
const read = (p) => readFileSync(join(root, p), 'utf8')

test('F-1 a pad can type into a Material text field: A opens the keyboard and the answer is written back with input and change events', async () => {
  const events = []
  const field = { localName: 'md-outlined-text-field', type: 'text', value: 'old', readOnly: false, disabled: false, maxLength: -1, getAttribute: (name) => (name === 'label' ? 'PIN' : null), dispatchEvent: (event) => { events.push(event.type); return true } }
  const done = editInputWithOsk(field)
  commitOsk('1234')
  assert.equal(await done, true)
  assert.equal(field.value, '1234')
  assert.deepEqual(events, ['input', 'change'])
  assert.ok(read('DesktopApp.vue').includes('isTextEntryElement(focused)'), 'A on a focused field goes to the keyboard, not to a click')
})

test('F-2 the snackbar host is mounted in the big-screen window, so a result said in a snackbar (a refused panel page, as the driver messages were) shows', () => {
  assert.ok(read('DesktopApp.vue').includes('<MNotifyHost />'))
  assert.ok(read('components/PanelLink.vue').includes('useNotify().snack('), 'what says its result in a snackbar is read')
})

test('F-3 unpair posts JSON and goes by the boolean status Sunshine returns', () => {
  const src = read('views/DevicesView.vue')
  const unpair = src.slice(src.indexOf('async function unpairDevice'), src.indexOf('// === QR 配对'))
  assert.ok(unpair.includes("headers: { 'Content-Type': 'application/json' }"))
  assert.ok(unpair.includes("data.status?.toString() === 'true'"))
  assert.ok(!unpair.includes("'ok'"))
})

test('F-4 the pet switch no longer lives in the big screen (a row opens the panel); in the panel the switch stays off until consent is given', () => {
  assert.ok(!existsSync(join(root, 'components/settings/SettingsPetCard.vue')))
  assert.ok(read('panelLinks.js').includes("pet: { page: 'pet'"))
  const panel = readFileSync(join(root, '../components/m3e/PanelOptions.vue'), 'utf8')
  assert.ok(panel.includes('const togglePet = (on) => { if (!on) stopObserving(); else consent.value = true }'), 'switching on asks first')
  assert.ok(panel.includes('const acceptPet = () => { consent.value = false; startObserving() }'), 'only a yes starts it')
})

test('F-5 nothing in the Chinese texts is left in English, except names (codec, driver, controller, key names)', () => {
  const names = new Set(['.stream.codec.hevc', '.vddSettings.sdr10bit', '.vddSettings.hdr12bit', '.m3e.home.pad.x360', '.m3e.home.pad.ds4', '.deviceHub.eyebrow', '.controllersHub.eyebrow', '.controllersHub.emulation.xbox360', '.aiAssistant.apiKey'])
  const left = []
  const walk = (a, b, path) => {
    for (const key of Object.keys(a)) {
      const at = `${path}.${key}`
      if (a[key] && typeof a[key] === 'object') walk(a[key], (b || {})[key] || {}, at)
      else if (typeof a[key] === 'string' && a[key] === (b || {})[key] && /^[\x20-\x7e]+$/.test(a[key]) && /[A-Za-z]{3}/.test(a[key]) && /\s/.test(a[key]) && !names.has(at)) left.push(`${at} = ${a[key]}`)
    }
  }
  walk(en, zh, '')
  assert.deepEqual(left, [])
  assert.equal(zh.settings.logLevel, '日志级别')
})

test('F-6 no native alert, confirm or prompt is left in the big-screen window: a pad cannot answer them', () => {
  const files = []
  const walk = (dir) => { for (const name of readdirSync(dir)) { const path = join(dir, name); if (statSync(path).isDirectory()) walk(path); else if (/\.(vue|js)$/.test(name) && !/\.test\.js$/.test(name)) files.push(path) } }
  walk(root)
  const hits = []
  for (const file of files) {
    const lines = readFileSync(file, 'utf8').split('\n')
    lines.forEach((line, i) => { if (/(^|[^.\w])(window\.)?(alert|confirm|prompt)\(/.test(line) && !/^\s*(\/\/|\*)/.test(line)) hits.push(`${file.slice(root.length)}:${i + 1}`) })
  }
  assert.deepEqual(hits, [])
})
