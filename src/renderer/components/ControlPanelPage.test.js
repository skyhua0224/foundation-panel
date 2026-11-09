import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'

const page = readFileSync(new URL('./ControlPanelPage.vue', import.meta.url), 'utf8')
const m3e = (f) => readFileSync(new URL(`../../../node_modules/foundation-ui/src/${f}`, import.meta.url), 'utf8')

// 软件更新: choosing Mirror酱 opens the CDK field inside the same MGroup as the other rows. The field must be a row of the group (key icon inside, no loose outlined box)
const updateGroup = page.slice(page.indexOf(":title=\"t.sidebar.softwareUpdate\""), page.indexOf('<PanelOptions'))
const mirrorBlock = updateGroup.slice(updateGroup.indexOf("updateSource.source.value === 'mirror'"))

test('the Mirror酱 CDK field sits in the one group of 软件更新 and carries its key icon', () => {
  assert.equal((updateGroup.match(/<MGroup>/g) || []).length, 1, 'one group, not pieces')
  assert.match(mirrorBlock, /<MFieldItem v-model="cdkDraft" icon="key" type="password"/)
})

test('the group package makes a field among rows a row of the group (card, corners, 2 px seam)', () => {
  const css = m3e('css/notify.css')
  assert.match(css, /\.m-group:has\(> \.m-item\):has\(> \.m-fieldrow\) \{ gap: 2px; \}/)
  assert.match(css, /\.m-group:has\(> \.m-item\) > \.m-fieldrow \{ background: var\(--m-card\)/)
  assert.match(m3e('vue/MFieldItem.vue'), /slot="leading-icon"/)
})

test('the CDK status row lets its three buttons drop below the text on a narrow panel', () => {
  assert.match(mirrorBlock, /<MItem icon="verified_user" wrap /)
  assert.match(m3e('vue/MItem.vue'), /wrap: \{ type: Boolean/)
})

test('the Panel has no 退出登录 row: the WebUI keeps its own log out in a browser', () => {
  assert.doesNotMatch(page, /logout/i)
  assert.doesNotMatch(page, /#logout/)
})
