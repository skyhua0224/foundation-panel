import assert from 'node:assert/strict'
import test from 'node:test'
import { readFileSync } from 'node:fs'

const root = new URL('../../../', import.meta.url)
const read = (path) => readFileSync(new URL(path, root), 'utf8')

test('the RTSS entry of the tray tools menu exists only where RivaTuner is installed', () => {
  const menu = read('src-tauri/src/tray/menu.rs')
  const rtss = read('src-tauri/src/rtss.rs')
  assert.match(rtss, /pub\(crate\) fn is_rtss_installed\(\) -> bool/)
  assert.match(menu, /if crate::rtss::is_rtss_installed\(\) \{\s*Some\(MenuItem::with_id\(app, "rtss_control"/)
  assert.match(menu, /if let Some\(rtss_control\) = &rtss_control \{\s*tools_items\.push\(rtss_control\)/)
  assert.doesNotMatch(menu, /vec!\[&show_toolbar, &host_performance, &rtss_control\]/)
})

test('native About, pairing, and log entries target the main shell pages', () => {
  const rust = read('src-tauri/src/windows.rs')
  const parts = read('src/renderer/composables/panelParts.js')
  const diagnostics = read('src/renderer/components/m3e/DiagnosticsPage.vue')
  const commands = read('src-tauri/src/commands.rs')
  const toolbar = read('src-tauri/src/toolbar.rs')
  const pet = read('src/renderer/toolbar/ToolbarApp.vue')
  const petMain = read('src/renderer/toolbar/main.js')
  const toolWindow = read('src/renderer/tool-window/ToolWindow.vue')
  const updater = read('src-tauri/src/update.rs')
  const nrControls = read('src/renderer/components/NrSessionControls.vue')
  const desktopMain = read('src/renderer/desktop/main.js')
  const desktopStyle = read('src/renderer/desktop/desktop.less')
  assert.doesNotMatch(rust, /(?:about|console|pin)\/index\.html/)
  assert.equal(readFileExists('src/renderer/about/index.html'), false)
  assert.equal(readFileExists('src/renderer/pin/index.html'), false)
  assert.equal(readFileExists('src/renderer/console/index.html'), false)
  assert.equal(readFileExists('src/renderer/stop-clock-canvas/index.html'), false)
  assert.equal(readFileExists('src/renderer/public/console/drag-drop-demo.html'), true)
  assert.doesNotMatch(read('vite.config.js'), /(?:about|pin): resolve\(rendererSrcPath/)
  assert.doesNotMatch(read('vite.config.js'), /console: resolve\(rendererSrcPath/)
  assert.doesNotMatch(read('vite.config.js'), /clock: resolve\(rendererSrcPath/)
  assert.match(parts, /about: \{ route: 'about'/)
  assert.match(parts, /pairing: \{ route: 'web-clients'/)
  assert.match(parts, /logs: \{ route: 'diagnostics'/)
  assert.match(parts, /pairing: '\/pin#pairing'/)
  assert.match(parts, /logs: '\/troubleshooting'/)
  for (const operation of ['refreshLogs', 'exportLogs', 'clearLogs']) assert.match(diagnostics, new RegExp(`function ${operation}`))
  assert.match(commands, /"nr" => windows::open_main_panel_page\(&app, "quality"\)/)
  assert.match(commands, /"performance" \| "host_performance" => windows::open_main_panel_page\(&app, "live-session"\)\?/)
  assert.match(toolbar, /"performance" \| "toolbar_performance" => open_panel_page\(app, "live-session"\)/)
  assert.match(toolbar, /"rtss" \| "toolbar_rtss" => open_panel_page\(app, "rtss"\)/)
  assert.match(pet, /pixiCanvas/)
  assert.match(pet, /toggleMenu/)
  assert.match(pet, /--pet-surface/)
  assert.match(petMain, /foundation-ui\/all\.css/)
  assert.match(toolWindow, /NrOverlayTool\.vue/)
  for (const oldTool of ['BitrateTool.vue', 'DpiAdjusterTool.vue', 'HostPerformanceTool.vue', 'RtssOsdTool.vue', 'ShortcutsTool.vue', 'PetSettingsTool.vue']) {
    assert.equal(readFileExists(`src/renderer/tool-window/tools/${oldTool}`), true, `${oldTool} is retained for the desktop compatibility surface`)
  }
  assert.doesNotMatch(updater, /SimSun/)
  assert.match(updater, /Segoe UI Variable Text/)
  assert.match(updater, /Microsoft YaHei UI/)
  assert.match(updater, /updater_dark_theme/)
  assert.doesNotMatch(nrControls, /<el-switch|element-plus/)
  assert.match(nrControls, /<MSwitchItem/)
  assert.match(desktopMain, /foundation-ui\/all\.css/)
  assert.match(desktopStyle, /var\(--md-sys-color-surface/)
})

function readFileExists(path) {
  try { readFileSync(new URL(`../../../${path}`, import.meta.url)); return true } catch { return false }
}
