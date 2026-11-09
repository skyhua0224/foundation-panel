import test from 'node:test'
import assert from 'node:assert/strict'
import { installedOf, noteRtssStatus, readRtssInstall, rtssInstalled } from './rtssInstall.js'
import { readFileSync } from 'node:fs'
import { requestPanelPage } from './panelParts.js'

// RivaTuner Statistics Server is an optional third-party component: its page exists only where get_rtss_status says installed
// (panelNav.js and useRouter.js import the UI framework, so they are read as source here)
const ROUTES = Object.fromEntries([...readFileSync(new URL('./useRouter.js', import.meta.url), 'utf8').matchAll(/^  ([A-Z_]+): '([\w-]+)',/gm)].map((m) => [m[1], m[2]]))
const nav = readFileSync(new URL('./panelNav.js', import.meta.url), 'utf8')
const menu = readFileSync(new URL('../components/SidebarMenu.vue', import.meta.url), 'utf8')

test('installed is the field of the host; a build without the field is judged by what it found', () => {
  assert.equal(installedOf({ installed: true, running: false }), true)
  assert.equal(installedOf({ installed: false, running: true }), false)
  assert.equal(installedOf({ running: true }), true)
  assert.equal(installedOf({ cli_path: 'C:\\rtss-cli.exe' }), true)
  assert.equal(installedOf({ running: false, cli_path: '' }), false)
  assert.equal(installedOf(null), false)
})

test('the row of RTSS is in the list only when installed (not read yet: not offered); installed but not running keeps it, the page shows the start hint', () => {
  assert.match(nav, /\.\.\.\(runtime\.rtssInstalled \? \[\{ id: ROUTES\.RTSS,/)
  assert.match(menu, /rtssInstalled: rtssInstalled\.value === true,/, 'the menu passes installed (null counts as not installed)')
  assert.match(menu, /rtss: computed\(\(\) => \(\{ installed: rtssInstalled\.value === true \}\)\)/, 'the row lines and the 系统 line drop RTSS too')
  assert.match(menu, /installed === false && route === ROUTES\.RTSS\) router\.navigate\(ROUTES\.SUNSHINE_SERVICE\)/)
  assert.doesNotMatch(readFileSync(new URL('../components/m3e/RtssPage.vue', import.meta.url), 'utf8'), /installed/, 'the page itself only shows running or not: installed is decided before it')
})

test('the status is read once; a failed read is not installed; a refresh that finds it gone closes the page', async () => {
  let calls = 0
  assert.equal(await readRtssInstall(async () => { calls++; return { installed: true, running: false } }), true)
  assert.equal(calls, 1)
  noteRtssStatus({ installed: false, running: false })
  assert.equal(rtssInstalled.value, false)
  assert.equal(await readRtssInstall(async () => { throw new Error('no such command') }), false)
})

test('a failed re-read keeps an installed answer', async () => {
  noteRtssStatus({ installed: true, running: false })
  assert.equal(await readRtssInstall(async () => { throw new Error('the command timed out') }), true)
  assert.equal(rtssInstalled.value, true)
})

test('a page link to RTSS is refused where it is known not installed', async () => {
  noteRtssStatus({ installed: false })
  assert.equal(requestPanelPage('rtss'), null)
  noteRtssStatus({ installed: true })
  assert.equal(requestPanelPage('rtss'), ROUTES.RTSS)
})
