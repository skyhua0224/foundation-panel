import assert from 'node:assert/strict'
import fs from 'node:fs'
import test from 'node:test'

// How the frame's moves reach the panel's route (the shell and the frame need the bundler and the window, so the wiring is read from the source; the rules themselves are in frameFollow.test.js and navSelection.test.js)
const frame = fs.readFileSync(new URL('../components/SunshineFrame.vue', import.meta.url), 'utf8')
const shell = fs.readFileSync(new URL('../components/SidebarMenu.vue', import.meta.url), 'utf8')
const inject = fs.readFileSync(new URL('../../../src-tauri/inject-script.js', import.meta.url), 'utf8')

test('every address the frame reports (a loaded document, a link, a hash) is handed to the shell, which makes the route follow it', () => {
  const pathUpdate = frame.slice(frame.indexOf("'path-update'"), frame.indexOf("'navigation-start'"))
  assert.match(pathUpdate, /currentPath\.value = data\.path/)
  assert.match(pathUpdate, /sidebarMenuRef\.value\?\.followFrame\?\.\(data\.path\)/)
  const start = frame.slice(frame.indexOf("'navigation-start'"), frame.indexOf("'webui-ready'"))
  assert.match(start, /sidebarMenuRef\.value\?\.followFrame\?\.\(data\.path\)/)
  assert.match(start, /openWelcome\(\)[\s\S]*return[\s\S]*followFrame/, 'the welcome page is the panel\'s own and is never followed')
})

test('the shell follows the frame only while the frame is what is displayed, and as a jump (Back returns to the page the link was on)', () => {
  const body = shell.slice(shell.indexOf('const followFrame = '), shell.indexOf('const followFrame = ') + 700)
  assert.match(body, /if \(!isWebPage\.value\) return/)
  assert.match(body, /framedMove\(path, \{ route: router\.currentRoute\.value, params: nav\.detail\.value\?\.params \}/)
  assert.match(body, /jumps\.jump\(\(\) => router\.navigate\(move\.route, move\.params\)\)/)
  assert.match(shell, /goHome, checkForUpdates: handleCheckForUpdates, router, followFrame,/)
})

test('the script the proxy puts into every page reports a place set by the page\'s hash as well as a new document', () => {
  const nav = inject.slice(inject.indexOf('const initNavigation'), inject.indexOf('// What happens inside the page is reported'))
  assert.match(nav, /if \(e\.hashChange\) \{/)
  assert.match(nav, /postToParent\('navigation-start', \{ path: full \}\)/)
  assert.doesNotMatch(nav, /e\.hashChange \|\| e\.downloadRequest/, 'a hash change is no longer dropped')
})

test('the moves the product makes for the person are jumps: search actions, the version line, status notices, the offline page, the connect dialog', () => {
  for (const line of [
    /exportDiag: \(\) => jumpTo\(ROUTES\.DIAGNOSTICS\)/,
    /logs: \(\) => jumpTo\(ROUTES\.DIAGNOSTICS\)/,
    /addApp: \(\) => jumps\.jump\(/,
    /scanGames: \(\) => jumps\.jump\(/,
    /const openUpdates = \(\) => \{ jumpTo\(ROUTES\.PANEL_SETTINGS\) \}/,
    /components: \(\) => jumpTo\(ROUTES\.COMPONENTS\), microphone: \(\) => jumpTo\(ROUTES\.MICROPHONE\), vdd: \(\) => jumpTo\(ROUTES\.VDD_SETTINGS\), update:/,
    /@logs="jumpTo\(ROUTES\.DIAGNOSTICS\)"/,
    /@open-downloads="\(\) => \{ connect\.open\.value = false; jumps\.jump\(\(\) => openRoute\(ROUTES\.WEB_DOWNLOADS\)\) \}"/,
  ]) assert.match(shell, line)
})

test('a frame that a link moved is loaded again when the panel asks for the address it remembers (Back to the page the link was on)', () => {
  assert.match(frame, /let frameMovedAway = false/)
  assert.match(frame, /const again = sunshineUrl\.value === url && frameMovedAway/)
  assert.match(frame, /if \(again\) \{ frameKey\.value\+\+; frameGoReady = false \} else sunshineUrl\.value = url/)
  // every reported address is compared with the one the panel gave the frame, and every request for an address goes through the helper
  assert.equal((frame.match(/notePath\(data\.path\)/g) || []).length, 2)
  assert.match(frame, /beginLoading\(\)\s*\n\s*setFrameUrl\(url\)/)
  assert.match(frame, /if \(next\.url\) setFrameUrl\(next\.url\)/)
  assert.match(frame, /setFrameUrl\(decision\.url\)/)
})
