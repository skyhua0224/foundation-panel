import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createJumpHistory } from '../../../node_modules/foundation-ui/src/nav/jumpHistory.js'
import { noticeView, afterCheck, afterOpened } from '../../../node_modules/foundation-ui/src/update/notice.js'

// the version in the caption bar, the update notice and the global back, as the panel wires them (the shell needs the bundler: its wiring is read from the source)
const sidebar = readFileSync(new URL('../components/SidebarMenu.vue', import.meta.url), 'utf8')

test('the caption bar carries the back button, the version and the update line; the chip is the scene\'s', () => {
  assert.match(sidebar, /<template v-if="backButton" #back><MCaptionBack :page="backPage"/)
  assert.match(sidebar, /<template #version><MCaptionVersion :version="currentVersion" :build="buildInfo"[^>]*:update="notice\.version"[^>]*@open-update="openUpdates"/)
  assert.match(sidebar, /<MScene :nav="nav" :caption-height="48" :jumps="jumps" :chip="backChip"/)
  assert.match(sidebar, /<template #back-chip><MBackChip :page="backPage"/)
  assert.match(sidebar, /build info.*|sunshine\.getBuildInfo\(\)/)
})
test('the red dot is on the rail item that holds 软件更新 (面板设置) and on 更多 of the phone; nothing is on 帮助与关于 any more', () => {
  assert.match(sidebar, /item\.id === 'settings' && notice\.value\.dot \? \{ \.\.\.item, badge: 'dot' \}/)
  assert.match(sidebar, /item\.id === 'more' && notice\.value\.dot \? \{ \.\.\.item, badge: 'dot' \}/)
  assert.doesNotMatch(sidebar, /item\.id === 'help' && /)
  assert.match(sidebar, /:text="settingsList\.lines\.value\.panel" :tone="notice\.dot \? 'warn' : ''"/)
})
test('opening 软件更新 (面板设置, 关于, a person\'s own check) clears the dot; the sheet the updater opens by itself does not', () => {
  assert.match(sidebar, /const updatesOpen = computed\(\(\) => router\.currentRoute\.value === ROUTES\.PANEL_SETTINGS \|\| router\.currentRoute\.value === ROUTES\.ABOUT\)/)
  assert.doesNotMatch(sidebar, /updatesOpen = computed\(\(\) => showUpdateDialog/)
  assert.match(sidebar, /remember\(afterOpened\(afterCheck\(noticeState\.value, \{ version: result\.version, is_latest: result\.is_latest \}/)
  assert.match(sidebar, /watch\(updateInfo, \(info\) => \{ if \(info\) remember\(afterCheck\(/)
})
test('the notice shows nothing until the installed version is known and respects the skipped version', () => {
  assert.match(sidebar, /installedKnown = computed\(\(\) => \/\^\[vV\]\?\\d\/\.test\(String\(currentVersion\.value \|\| ''\)\) && currentVersion\.value !== '0\.0\.0'\)/)
  assert.match(sidebar, /noticeView\(noticeState\.value, \{ installed: currentVersion\.value, skipped: skippedVersion\.value \}\)/)
})
test('the jumps: pages and tiles open as jumps, the rail and the list rows do not', () => {
  for (const fn of ['goDiagnosticsPlace', 'goDoctorTarget', 'openDestination', 'onUnifiedChoose', 'onPendingGo']) assert.match(sidebar, new RegExp(`const ${fn} = \\([^)]*\\) => jumps\\.jump\\(\\(\\) => \\{`), fn)
  assert.match(sidebar, /const openDoctor = \(\) => jumps\.jump\(\(\) => router\.navigate\(ROUTES\.DOCTOR/)
  assert.match(sidebar, /@open-network="jumpConfig\(\{ tab: 'network' \}\)"/)
  const onRail = sidebar.slice(sidebar.indexOf('const onRail = '), sidebar.indexOf('const onSearch = '))
  assert.doesNotMatch(onRail, /jumps\./, 'the rail is the person\'s move')
  const onList = sidebar.split('\n').find((l) => l.startsWith('const onListNavigate'))
  assert.doesNotMatch(onList, /jumps\./)
  // a place is restored through the panel's own openers: a Web page also needs its frame told
  assert.match(sidebar, /const restorePlace = \(place\) => \{[\s\S]*?openConfig\(place\.params[\s\S]*?emit\('webui-navigate', path\)[\s\S]*?router\.navigate\(place\.id, place\.params\)/)
})
test('the search is its icon when the version leaves it no room (measured), so the bar never cuts the field', () => {
  assert.match(sidebar, /:compact="compactSearch \|\| caption\.tight\.value"/)
  assert.match(sidebar, /const caption = useCaptionRoom\(\)/)
})
test('the panel scenario: 体检 → 网络 → back lands on 体检 with its token; a rail move clears it; the notice walks through its four states', () => {
  const at = { value: { id: 'doctor', params: { autostart: 17 }, rail: 'system' } }
  const h = createJumpHistory({ locate: () => at.value, restore: (p) => { at.value = p } })
  h.jump(() => { at.value = { id: 'web-config', params: { tab: 'network' }, rail: 'config' } })
  assert.equal(h.canBack.value, true)
  h.back()
  assert.deepEqual(at.value, { id: 'doctor', params: { autostart: 17 }, rail: 'system' })
  let s = { available: '', seen: '' }
  const opts = { installed: '2026.1107.120000' }
  assert.equal(noticeView(s, opts).waiting, false)
  s = afterCheck(s, { version: '2026.1109.090000' }, opts); assert.deepEqual(noticeView(s, opts), { waiting: true, version: '2026.1109.090000', dot: true })
  s = afterOpened(s); assert.deepEqual(noticeView(s, opts), { waiting: true, version: '2026.1109.090000', dot: false })
  s = afterCheck(s, { version: '2026.1112.080000' }, opts); assert.equal(noticeView(s, opts).dot, true)
})
