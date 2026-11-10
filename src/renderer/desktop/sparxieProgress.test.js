import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'

const read = (p) => readFileSync(new URL(`../${p}`, import.meta.url), 'utf8')
const install = read('composables/sparxieInstall.js')
const status = read('composables/useComponentStatus.js')

test('the install line on the Sparxie card is the card\'s own state and always ends with the install (「正在解压」 stayed after a finished install)', () => {
  assert.ok(!/sparxieProgress/.test(install + status), 'no module-level flag that a read of the host\'s status copies into the answer')
  assert.ok(!/body\.progress/.test(status), 'a read of the status carries no progress')
  assert.ok(status.includes("progress: (value) => controller.setProgress('sparxie', value)"))
  const run = install.slice(install.indexOf('import: {'), install.indexOf('open: {'))
  assert.ok(/finally \{\s*progress\(null\)\s*await refresh\(\)/.test(run), 'cleared in finally: a failed request, a refusal and a cancel end it too')
  assert.ok(/refresh: \(\) => controller\.refresh\(\['sparxie'\]\)/.test(status), 'the last read is a new one, not a check that returns early while an older read is in flight')
})

test('打开 Sparxie on the card starts it with the panel\'s own command, never through the host service', () => {
  const open = install.slice(install.indexOf('open: {'), install.indexOf('remove: {'))
  assert.ok(open.includes("invoke('open_sparxie', { exe: status.exe })"))
  assert.ok(!/\/api\/sparxie\/open/.test(open), 'the service is not asked to start a window')
  assert.ok(open.includes('status.running') && open.includes('alreadyOpen'))
})
