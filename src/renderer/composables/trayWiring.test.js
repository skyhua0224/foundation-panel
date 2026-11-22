import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { installTrayTheme } from './trayTheme.js'
import { trayTransport } from './menuPopupTransport.js'
import { SLIDER_KEYS, TILE_IDS, TRAY_DENSITIES, TRAY_ITEM_IDS, VARIANTS, modelIds, normalizeLayout, openingDensity, tileItemIds } from 'foundation-ui/tray/trayModel.js'
import { TRAY_PREFS_DEFAULT } from 'foundation-ui/tray/trayPrefs.js'

// The tray menus in the menu window (docs/tray-menu.md, src-tauri/src/tray/popup_input.rs): the ids, the commands and events both sides name, the settings page, the theme the menus open with.
const read = (p) => readFileSync(new URL(`../../../${p}`, import.meta.url), 'utf8')
const menuRs = read('src-tauri/src/tray/menu.rs')
const actionsRs = read('src-tauri/src/tray/actions.rs')
const inputRs = read('src-tauri/src/tray/popup_input.rs')
const ctxRs = read('src-tauri/src/ctx_popup.rs')
const mainRs = read('src-tauri/src/main.rs')
const trayRs = read('src-tauri/src/tray.rs')

const menuIds = [...menuRs.matchAll(/(?:MenuItem|CheckMenuItem)::with_id\(\s*app,\s*"([a-z_0-9]+)"/g)].map((m) => m[1])
const rustIds = [...inputRs.slice(inputRs.indexOf('ITEM_IDS: [&str; 46]')).matchAll(/"([a-z_0-9]+)"/g)].map((m) => m[1]).slice(0, 46)

test('the 46 item ids are the same in menu.rs, the popup\'s list in Rust, the handler arms and the menus\' model', () => {
  assert.equal(rustIds.length, 46)
  // The containers are submenus; the item IDs themselves must match.
  assert.deepEqual([...new Set(menuIds)].sort(), [...rustIds].sort())
  assert.deepEqual([...TRAY_ITEM_IDS].sort(), [...rustIds].sort())
  for (const id of rustIds.filter((x) => x !== 'tray_status')) assert.ok(actionsRs.includes(`"${id}" =>`), `no handler arm for ${id}`)
})

for (const [name, model] of Object.entries(VARIANTS)) {
  test(`${name}: every id of the native menu can be reached in the menu, and the ids it sends are accepted by the host`, () => {
    const reached = modelIds(model)
    for (const id of rustIds) assert.ok(reached.has(id), `${name} cannot reach ${id}`)
    for (const id of reached.keys()) assert.ok(rustIds.includes(id), `${name} sends ${id}, which the host does not accept`)
  })
}

test('an item chosen in a menu runs through handle_tray_menu_event, after the window closed it; unknown ids are refused', () => {
  assert.match(inputRs, /handle_tray_menu_event\(app, id\)/)
  assert.match(inputRs, /if !ITEM_IDS\.contains\(&id\)/)
  assert.ok(inputRs.indexOf('ctx_popup::pick_tray') < inputRs.indexOf('handle_tray_menu_event(app, id)'))
})

test('the commands and events: both sides name the same ones, and main.rs registers every command', () => {
  for (const cmd of ['tray_prefs_get', 'tray_prefs_set']) { assert.ok(mainRs.includes(`tray::${cmd}`), cmd); assert.ok(trayRs.includes(`pub fn ${cmd}`), cmd) }
  // the page's item runs through an ASYNC command: a synchronous one runs on the main thread inside the page's own WebView2 callback, and the big screen built there never returned (the panel froze)
  assert.ok(mainRs.includes('tray::tray_popup_act')); assert.ok(trayRs.includes('pub async fn tray_popup_act')); assert.ok(!trayRs.includes('pub fn tray_popup_act'))
  assert.ok(mainRs.includes('ctx_popup::tray_popup_theme_set')); assert.ok(ctxRs.includes('pub fn tray_popup_theme_set'))
  const calls = []
  const t = trayTransport({ invoke: async (...a) => { calls.push(a) }, listen: (name) => name })
  assert.equal(t.onOpen(() => {}), 'tray-popup:open'); assert.equal(t.onState(() => {}), 'tray-popup:state'); assert.equal(t.onClose(() => {}), 'ctx-popup:close')
  for (const ev of ['tray-popup:open', 'tray-popup:state', 'ctx-popup:close']) assert.ok(ctxRs.includes(`"${ev}"`), ev)
  t.ready(7, { w: 304, h: 400 }); t.act(7, 'about'); t.cancel(7)
  assert.deepEqual(calls, [['ctx_popup_ready', { token: 7, width: 304, height: 400 }], ['tray_popup_act', { token: 7, id: 'about' }], ['ctx_popup_cancel', { token: 7 }]])
})

test('the menu window page shows both menus; the tray page does not tell the host the window loaded twice', () => {
  const main = read('src/renderer/ctxmenu/main.js')
  assert.match(main, /MMenuPopup, MTrayPopup/); assert.match(main, /h\(MTrayPopup, \{ transport: tray \}\)/)
  assert.match(read('src/renderer/composables/menuPopupTransport.js'), /loaded: \(\) => \{\}/)
})

test('the icon\'s events go through popup_input (the three gestures are the person\'s choice), the native menu is the fallback, and the single and double click are not both fired', () => {
  assert.match(trayRs, /on_tray_icon_event\(\|_tray, event\| popup_input::on_icon_event\(event\)\)/)
  assert.match(inputRs, /FAILURES_BEFORE_NATIVE/); assert.match(inputRs, /force_native/)
  assert.match(inputRs, /set_menu\(None::<Menu<R>>\)/)
  assert.match(trayRs, /if popup_input::use_native_menu\(\)/, 'a rebuild does not give the icon its native menu back while the popup is in use')
  assert.match(inputRs, /just_closed_by_icon_press/)
})

test('the settings page edits the record the host keeps, and its defaults are the host\'s', () => {
  const page = read('src/renderer/components/m3e/AppearancePage.vue')
  assert.match(page, /MTraySettings/); assert.match(page, /tray_prefs_get/); assert.match(page, /tray_prefs_set/)
  const rust = read('src-tauri/src/tray_prefs.rs')
  assert.match(rust, new RegExp(`right: "${TRAY_PREFS_DEFAULT.right}"\\.into\\(\\), click: "${TRAY_PREFS_DEFAULT.click}"\\.into\\(\\), double: "${TRAY_PREFS_DEFAULT.double}"\\.into\\(\\), force_native: ${TRAY_PREFS_DEFAULT.forceNative}`))
})

test('the menus open with the colours of the main window: it reports its theme after a burst of changes, once per distinct theme', async () => {
  const sent = []
  let observer = null
  class Fake { constructor(cb) { this.cb = cb; observer = this } observe() {} disconnect() { this.gone = true } }
  const timers = []
  let theme = { sheet: 'a' }
  const doc = { documentElement: {}, head: {} }
  const stop = installTrayTheme({ invoke: async (...a) => { sent.push(a) }, doc, snapshot: () => theme, setTimer: (fn) => timers.push(fn) - 1, clearTimer: () => {}, Observer: Fake })
  assert.equal(timers.length, 1, 'reports once at start')
  timers[0](); assert.deepEqual(sent, [['tray_popup_theme_set', { theme: { sheet: 'a' } }]])
  observer.cb(); observer.cb(); assert.equal(timers.length, 2, 'a burst is one report')
  timers[1](); assert.equal(sent.length, 1, 'the same theme is not sent again')
  theme = { sheet: 'b' }; observer.cb(); timers[2](); assert.equal(sent.length, 2)
  stop(); assert.ok(observer.gone)
})

test('the sliders: tray_popup_set is registered, its answer reaches the page (a refusal too), and the host takes exactly the four keys', async () => {
  const mainRs = read('src-tauri/src/main.rs'), trayRs = read('src-tauri/src/tray.rs'), sliderRs = read('src-tauri/src/tray/display_sliders.rs')
  assert.ok(mainRs.includes('tray::tray_popup_set')); assert.ok(trayRs.includes('pub async fn tray_popup_set')); assert.ok(trayRs.includes('mod display_sliders;'))
  const calls = []
  const ok = trayTransport({ invoke: async (...a) => { calls.push(a); return 'done' }, listen: () => {} })
  assert.equal(await ok.set(7, 'dpi', 150), 'done'); assert.deepEqual(calls, [['tray_popup_set', { token: 7, key: 'dpi', value: 150 }]])
  const refused = trayTransport({ invoke: async () => { throw '显示器不支持这个模式' }, listen: () => {} })
  await assert.rejects(() => refused.set(7, 'refresh', 144), /显示器不支持这个模式/)
  for (const key of ['resolution', 'refresh', 'dpi', 'bitrate']) assert.ok(inputRs.includes(`"${key}" =>`), `popup_input::set has no arm for ${key}`)
  // the host is the one that writes: the same lock and config round trip as the other config writers, DisplayConfig (dpi_scale::set_scale) for the scale, Win32 for the modes
  assert.match(inputRs, /DEVICE_CONFIG_LOCK[\s\S]*read_full_sunshine_config[\s\S]*post_sunshine_config/); assert.ok(inputRs.includes('crate::dpi_scale::set_scale(scale_display().await, dpi)'))
  assert.ok(sliderRs.includes('ChangeDisplaySettingsExW') && sliderRs.includes('CDS_TEST') && sliderRs.includes('EnumDisplaySettingsExW'))
  // the page sends exactly the keys the model draws
  assert.deepEqual([...SLIDER_KEYS], ['resolution', 'refresh', 'dpi', 'bitrate'])
  assert.equal(VARIANTS.v3.pages.quick.rows.find((r) => r.t === 'sliders').source, 'layout', 'which sliders: the person\'s layout, the bitrate cap and the scale by default')
})

test('the quick panel\'s layout: the host keeps it apart from the click settings, accepts exactly the ids the menu knows, and sends it with the state', async () => {
  const rust = read('src-tauri/src/tray_prefs.rs')
  // the id lists of Rust and of foundation-ui are one list
  const rustList = (name) => [...rust.slice(rust.indexOf(name)).split('];')[0].matchAll(/"([a-z_0-9]+)"/g)].map((m) => m[1])
  assert.deepEqual(rustList('pub const TILE_IDS'), [...TILE_IDS]); assert.deepEqual(rustList('pub const SLIDER_KEYS'), [...SLIDER_KEYS])
  assert.ok(rust.includes('pub const TILE_IDS: [&str; 17]') && TILE_IDS.length === 17)
  // every tile is an item the handler runs (nothing invented)
  for (const id of TILE_IDS) for (const act of tileItemIds(id)) assert.ok(actionsRs.includes(`"${act}" =>`), `tile ${id}: no handler arm for ${act}`)
  // commands: registered, and the transport sends the record by name
  for (const cmd of ['tray_layout_set', 'tray_popup_layout']) { assert.ok(mainRs.includes(`tray::${cmd}`), cmd); assert.ok(trayRs.includes(`pub `) && trayRs.includes(`fn ${cmd}`), cmd) }
  const calls = []
  const t = trayTransport({ invoke: async (...a) => { calls.push(a) }, listen: () => {} })
  await t.layout(7, { tiles: ['vdd', 'auto_start'], sliders: null }); await t.layout(7, null)
  assert.deepEqual(calls, [['tray_popup_layout', { token: 7, tiles: ['vdd', 'auto_start'], sliders: null }], ['tray_popup_layout', { token: 7, tiles: null, sliders: null }]])
  // the settings page saves the click record without the layout, and the layout by its own command; the state carries the layout and the toolbar's state
  assert.match(trayRs, /save_keeping_layout\(prefs\)/); assert.match(read('src/renderer/components/m3e/AppearancePage.vue'), /tray_layout_set/)
  assert.match(inputRs, /"layout": \{ "tiles": prefs\.tiles, "sliders": prefs\.sliders \}/); assert.match(inputRs, /"show_toolbar": app\.get_webview_window\("toolbar"\)\.is_some\(\)/)
  assert.match(inputRs, /tray_prefs::save_layout\(tiles, sliders\)\?;[\s\S]*update_tray_state/)
  // what a stored record means to the menu
  assert.deepEqual(normalizeLayout({ tiles: null, sliders: null }).sliders, ['bitrate', 'dpi'])
  assert.deepEqual(normalizeLayout({ tiles: null, sliders: ['dpi'] }).sliders, ['dpi'], 'a list saved before the default changed is kept as it was saved')
})

test('the sliders are in all three menus from one list, and the native menu has stepped stand-ins for them from the same list and the same writer', () => {
  for (const v of ['v1', 'v2', 'v3']) {
    const rows = (v === 'v1' ? VARIANTS.v1.pages.root : v === 'v2' ? VARIANTS.v2.pages.root : VARIANTS.v3.pages.quick).rows
    assert.equal(rows.filter((r) => r.t === 'sliders' && r.source === 'layout').length, 1, v)
  }
  const menuRs = read('src-tauri/src/tray/menu.rs'), prefsRs = read('src-tauri/src/tray_prefs.rs')
  // one writer: the quick page and the native steps both end in write_slider (the config key max_bitrate, the display's scale through DisplayConfig)
  assert.match(inputRs, /async fn write_slider\(key: &str, value: &Value\)/); assert.match(inputRs, /let \(settle, notice\) = write_slider\(key, value\)\.await\?;/); assert.match(inputRs, /match write_slider\(key, &json!\(value\)\)\.await/)
  // the steps: the menu builds ids the handler parses; the labels are in three languages
  assert.match(menuRs, /format!\("slider_bitrate_\{mbps\}"\)/); assert.match(menuRs, /format!\("slider_dpi_\{pct\}"\)/)
  assert.match(menuRs, /popup_input::native_slider_keys\(\)/); assert.match(actionsRs, /_ if popup_input::native_slider_event\(app, menu_id\) => \{\}/)
  for (const key of ['bitrate_menu', 'bitrate_auto', 'dpi_menu']) assert.equal((read('src-tauri/src/tray.rs').match(new RegExp(`^    ${key}: "`, 'gm')) || []).length, 3, key)
  // the host record keeps its shape (tiles, sliders: null = the default), so a record saved by an older build still reads
  assert.match(prefsRs, /pub sliders: Option<Vec<String>>/); assert.match(inputRs, /unwrap_or_else\(\|\| vec!\["bitrate"\.to_string\(\), "dpi"\.to_string\(\)\]\)/)
})

test('touch density: the host tells the menu what opened it and the person\'s choice, one rule on both sides, an old record still reads', () => {
  const src = read('src-tauri/src/tray/input_source.rs'), prefs = read('src-tauri/src/tray_prefs.rs')
  // the densities of the record are the densities of the menu
  const list = [...prefs.slice(prefs.indexOf('pub const DENSITIES')).split('];')[0].matchAll(/"([a-z]+)"/g)].map((m) => m[1])
  assert.deepEqual(list, [...TRAY_DENSITIES])
  assert.match(prefs, /density: "auto"\.into\(\)/); assert.match(prefs, /#\[serde\(rename_all = "camelCase", default\)\]/)
  // what the state carries, and where the icon's click is read (first thing in the callback, while the message is being dispatched)
  assert.match(inputRs, /"touch": input_source::facts\(\)\.to_json\(\)/); assert.match(inputRs, /"density": prefs\.density/)
  assert.match(inputRs, /input_source::note_icon_event\(\);\n    let Some\(app\) = ctx_popup::app\(\)/)
  // the Win32 calls named by the documentation, and the documented constants
  for (const k of ['GetCurrentInputMessageSource', 'GetMessageExtraInfo', '0x2003', 'GetSystemMetrics(95)', '0x04 => Source::Touch', '0x08 => Source::Pen', '0x02 => Source::Mouse']) assert.ok(src.includes(k), k)
  // the same rule in JS: a finger or a pen, else slate posture when the press is unknown, a digitizer alone never
  for (const [source, slate, capable, want] of [['touch', false, false, 'touch'], ['pen', false, true, 'touch'], ['mouse', true, true, 'standard'], ['unknown', true, true, 'touch'], ['unknown', false, true, 'standard']]) assert.equal(openingDensity('auto', { source, slate, capable }), want, `${source} ${slate} ${capable}`)
  assert.ok(src.includes('Source::Unknown if facts.slate => "touch"') && src.includes('Source::Touch | Source::Pen => "touch"'))
})
