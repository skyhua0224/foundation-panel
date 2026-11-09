import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'

const read = (p) => readFileSync(new URL(p, import.meta.url), 'utf8')

// 体检 keeps its result when the person follows a fix and comes back: the entry of the history must not tell the page to run again
test('every press of 开始体检 carries its own token, and the page gets it as it is', () => {
  const shell = read('./SidebarMenu.vue')
  assert.match(shell, /router\.navigate\(ROUTES\.DOCTOR, \{ autostart: Date\.now\(\) \}\)/)
  assert.doesNotMatch(shell, /autostart: true/)
  assert.match(shell, /:autostart="entry\.params\?\.autostart \|\| false"/)
  assert.doesNotMatch(shell, /!!entry\.params\?\.autostart/)
  assert.match(read('./m3e/DoctorPage.vue'), /autostart: \{ type: \[Boolean, Number\]/)
})
