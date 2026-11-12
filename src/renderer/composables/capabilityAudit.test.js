import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'

// Tauri v2 refuses a command that no capability grants to the window (and origin) that calls it: 「Command x not allowed by ACL」. This reads the sources and fails when a command that is invoked
// has no grant for its caller: the bundled pages (any window, via the generated capability) get every command except the LOCAL_ONLY ones, which need a hand-written capability of their window;
// the Web UI in the frame is a remote origin of the main window and gets only what src-tauri/webui-permissions.json lists.
const tauri = new URL('../../../src-tauri/', import.meta.url).pathname
const renderer = new URL('../', import.meta.url).pathname
const read = (f) => fs.readFileSync(f, 'utf8')
const walk = (dir, out = []) => {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    if (['node_modules', 'dist', 'target', '_harness'].includes(e.name)) continue
    const f = path.join(dir, e.name)
    if (e.isDirectory()) walk(f, out)
    else if (/\.(vue|js)$/.test(e.name) && !/\.test\.js$/.test(e.name)) out.push(f)
  }
  return out
}
const mainRs = read(`${tauri}src/main.rs`)
const handlerBlock = mainRs.slice(mainRs.indexOf('tauri::generate_handler!['))
const HANDLERS = new Set(handlerBlock.slice(0, handlerBlock.indexOf(']')).split('\n').map((l) => l.split('//')[0].trim().replace(/,$/, '')).filter(Boolean).map((l) => l.split('::').pop()))
const buildRs = read(`${tauri}build.rs`)
const LOCAL_ONLY = new Set([...buildRs.slice(buildRs.indexOf('LOCAL_ONLY_COMMANDS'), buildRs.indexOf('];')).matchAll(/"([a-z_]+)"/g)].map((m) => m[1]))
const commandOf = (permission) => permission.slice('allow-'.length).replaceAll('-', '_')
const CAPS = {} // command -> windows of the hand-written capabilities that grant it
for (const f of fs.readdirSync(`${tauri}capabilities`)) {
  if (f.includes('.generated.')) continue
  const cap = JSON.parse(read(`${tauri}capabilities/${f}`))
  for (const permission of cap.permissions) if (permission.startsWith('allow-')) (CAPS[commandOf(permission)] ||= []).push(...cap.windows)
}
const REMOTE = new Set(JSON.parse(read(`${tauri}webui-permissions.json`)).filter((p) => p.startsWith('allow-')).map(commandOf))
// the commands the Web UI itself invokes in the frame (src_assets/common/assets/web: window.__TAURI__.core.invoke and the save dialog); the Web repo's own test keeps this list true
const WEBUI_INVOKES = ['scan_directory_for_apps', 'scan_game_libraries', 'list_display_modes', 'set_display_mode', 'get_current_dpi', 'set_desktop_dpi', 'is_sunshine_running_in_user_mode', 'toggle_sunshine_mode',
  'launch_windows_hdr_calibration', 'set_vdd_keep_enabled', 'set_vdd_headless_create_enabled', 'refresh_sunshine_target', 'open_external_url', 'open_device_manager', 'save_text_file']

// the window a file of the renderer runs in
const windowOf = (file) => (file.includes('/renderer/toolbar/') ? 'toolbar' : file.includes('/renderer/desktop/') ? 'desktop' : 'main')
const invokes = []
for (const file of walk(renderer)) {
  const src = read(file)
  for (const m of src.matchAll(/\b\w*[iI]nvoke\(\s*['"`]([a-z][a-z0-9_]*)['"`]/g)) invokes.push({ command: m[1], file: path.relative(renderer, file), window: windowOf(file) })
}

test('the audit sees the renderer\'s commands (it is not vacuous)', () => {
  assert.ok(HANDLERS.size > 150 && LOCAL_ONLY.size >= 20, `${HANDLERS.size} handlers, ${LOCAL_ONLY.size} local-only`)
  assert.ok(invokes.length > 100, `${invokes.length} invokes found`)
  assert.ok(invokes.some((i) => i.command === 'probe_sunshine_host') && invokes.some((i) => i.command === 'open_local_path'))
})

test('every command the renderer invokes is registered, and a local-only one is granted to the window that calls it', () => {
  const unknown = [...new Set(invokes.filter((i) => !HANDLERS.has(i.command)).map((i) => `${i.command} (${i.file})`))]
  assert.deepEqual(unknown, [], `invoked but not in generate_handler!: ${unknown.join(', ')}`)
  const denied = invokes.filter((i) => LOCAL_ONLY.has(i.command) && !(CAPS[i.command] || []).includes(i.window)).map((i) => `${i.command} from the ${i.window} window (${i.file})`)
  assert.deepEqual([...new Set(denied)], [], `no capability grants these to their window: ${[...new Set(denied)].join('; ')}`)
})

test('every local-only command has a hand-written grant, and no remote grant (the Web UI never gets them)', () => {
  for (const command of LOCAL_ONLY) {
    assert.ok(HANDLERS.has(command), command)
    assert.ok((CAPS[command] || []).length > 0, `${command} is local-only and no capability grants it to any window`)
    assert.ok(!REMOTE.has(command), `${command} is local-only and is in webui-permissions.json`)
  }
})

test('every command the Web UI invokes in the frame is granted to its origin (the 「Command set_desktop_dpi not allowed by ACL」 class)', () => {
  const missing = WEBUI_INVOKES.filter((c) => !REMOTE.has(c))
  assert.deepEqual(missing, [], `the Web UI invokes these and webui-permissions.json does not list them: ${missing.join(', ')}`)
  for (const command of REMOTE) assert.ok(HANDLERS.has(command) && !LOCAL_ONLY.has(command), `${command}`)
})

test('where a Web checkout is at hand, its invoke surface is exactly the list above', (t) => {
  const web = [process.env.WEB_UI_SRC, new URL('../../../../web-small-fix/src_assets/common/assets/web', import.meta.url).pathname, new URL('../../../../rp-web/src_assets/common/assets/web', import.meta.url).pathname].find((d) => d && fs.existsSync(d))
  if (!web) return t.skip('no Web UI checkout next to this repository')
  const used = new Set()
  for (const file of walk(web)) for (const m of read(file).matchAll(/\b\w*[iI]nvoke\(\s*['"`]([a-z][a-z0-9_]*)['"`]/g)) used.add(m[1])
  const surprising = [...used].filter((c) => !WEBUI_INVOKES.includes(c))
  assert.deepEqual(surprising, [], `the Web UI invokes commands this audit does not know: add them to WEBUI_INVOKES and to webui-permissions.json (${surprising.join(', ')})`)
})
