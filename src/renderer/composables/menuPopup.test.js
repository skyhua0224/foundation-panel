import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { menuPresenter, presentMenu } from 'foundation-ui/contextmenu/presenter.js'
import { installMenuPopup } from './menuPopup.js'
import { popupTransport } from './menuPopupTransport.js'

// the context menu in a window of its own: the page that owns the menu (presenter) and the page of the menu window (transport), against a native side that is a fake
const src = (p) => readFileSync(new URL(p, import.meta.url), 'utf8')
const items = [{ id: 'copy', label: 'Copy', run() {} }, { id: 'size', label: 'Size', children: [{ id: 's', label: 'Small', run() {} }] }]

function native({ failOpen = false } = {}) {
  const listeners = new Map()
  const calls = []
  return {
    calls,
    listen: async (name, cb) => { listeners.set(name, cb); return () => listeners.delete(name) },
    emit: (name, payload) => listeners.get(name)?.({ payload }),
    invoke: async (cmd, args) => { calls.push([cmd, args]); if (failOpen && cmd === 'ctx_popup_open') throw new Error('no menu window') },
    has: (name) => listeners.has(name),
  }
}
const win = () => ({ devicePixelRatio: 1.5 })
const tick = () => new Promise((r) => setTimeout(r, 0))

test('a menu goes to the native side as data with the theme and the place, and comes back as the path of the chosen item of this page', async () => {
  const n = native()
  const w = win()
  const menu = installMenuPopup({ invoke: n.invoke, listen: n.listen, backLabel: () => 'Zurück', theme: () => ({ sheet: 'x' }), win: w, now: () => 100 })
  await tick()
  assert.equal(menuPresenter(), menu.present, 'the in-page menu hands its menus to it')
  const shown = presentMenu({ items, point: { x: 40, y: 50 }, pointer: true })
  await tick()
  const [cmd, args] = n.calls[0]
  assert.equal(cmd, 'ctx_popup_open')
  const req = args.request
  assert.equal(req.pointer, true); assert.equal(req.clientX, 40); assert.equal(req.clientY, 50); assert.equal(req.ratio, 1.5); assert.equal(req.back, 'Zurück'); assert.deepEqual(req.theme, { sheet: 'x' })
  assert.ok(Number.isSafeInteger(req.token))
  assert.deepEqual(req.items.map((i) => i.id), ['copy', 'size']); assert.ok(!JSON.stringify(req.items).includes('"run"'))
  assert.deepEqual(w.__ctxPopupTimings.map((t) => t.token), [req.token])
  // a result for another token is not ours
  n.emit('ctx-popup:result', { token: req.token + 7, path: [0] })
  n.emit('ctx-popup:result', { token: req.token, path: [1, 0] })
  const res = await shown
  assert.equal(res.handled, true); assert.equal(res.picked.id, 's'); assert.equal(typeof res.picked.run, 'function')
  menu.stop()
  assert.equal(menuPresenter(), null)
  assert.equal(n.has('ctx-popup:result'), false, 'stopped listening')
})

test('a menu that closed without a choice answers with no item; a native side that cannot show it leaves the menu to the page', async () => {
  const n = native()
  const menu = installMenuPopup({ invoke: n.invoke, listen: n.listen, theme: () => null, win: win(), now: () => 1 })
  await tick()
  const closed = presentMenu({ items, point: { x: 1, y: 1 } })
  await tick()
  n.emit('ctx-popup:result', { token: n.calls[0][1].request.token, path: null, reason: 'Cancel' })
  assert.deepEqual(await closed, { handled: true, picked: null })
  menu.stop()
  const bad = native({ failOpen: true })
  const menu2 = installMenuPopup({ invoke: bad.invoke, listen: bad.listen, theme: () => null, win: win(), now: () => 1 })
  assert.deepEqual(await presentMenu({ items, point: { x: 1, y: 1 } }), { handled: false })
  menu2.stop()
})

test('stopping while a menu is on gives the menu back to the page', async () => {
  const n = native()
  const menu = installMenuPopup({ invoke: n.invoke, listen: n.listen, theme: () => null, win: win(), now: () => 1 })
  await tick()
  const shown = presentMenu({ items, point: { x: 1, y: 1 } })
  await tick()
  menu.stop()
  assert.deepEqual(await shown, { handled: false })
})

test('the menus of the Web pages in the frame go through the same presenter; the point moves into this window and the answer goes to the frame', async () => {
  const n = native()
  const menu = installMenuPopup({ invoke: n.invoke, listen: n.listen, theme: () => null, win: win(), now: () => 1 })
  await tick()
  const posted = []
  const frame = { contentWindow: { postMessage: (m, origin) => posted.push([m, origin]) }, getBoundingClientRect: () => ({ left: 200, top: 48 }) }
  const event = (data) => ({ source: frame.contentWindow, origin: 'http://127.0.0.1:48090', data })
  assert.equal(menu.frameMessage(event({ type: 'foundation-menu-hello' }), frame), true)
  assert.deepEqual(posted[0], [{ type: 'foundation-menu-host' }, 'http://127.0.0.1:48090'])
  assert.equal(menu.frameMessage(event({ type: 'foundation-menu-open', id: 'f1', model: [{ id: 'a', label: 'A' }], point: { x: 10, y: 20 }, pointer: true }), frame), true)
  await tick(); await tick()
  assert.deepEqual(posted[1][0], { type: 'foundation-menu-ack', id: 'f1' })
  const req = n.calls.find(([cmd]) => cmd === 'ctx_popup_open')[1].request
  assert.equal(req.clientX, 210); assert.equal(req.clientY, 68); assert.equal(req.pointer, true)
  n.emit('ctx-popup:result', { token: req.token, path: [0] })
  await tick(); await tick()
  assert.deepEqual(posted[2], [{ type: 'foundation-menu-result', id: 'f1', handled: true, path: [0] }, 'http://127.0.0.1:48090'])
  menu.stop()
})

test('the transport of the menu window: events in, commands out, and a failing command never throws into the page', async () => {
  const n = native()
  const t = popupTransport({ invoke: n.invoke, listen: n.listen })
  const opened = []
  await t.onOpen((p) => opened.push(p))
  await t.onClose((p) => opened.push(['close', p]))
  n.emit('ctx-popup:open', { token: 5, items: [] }); n.emit('ctx-popup:close', { token: 5 })
  assert.deepEqual(opened, [{ token: 5, items: [] }, ['close', { token: 5 }]])
  await t.ready(5, { w: 240, h: 400 }); await t.pick(5, [2, 0]); await t.cancel(5); await t.loaded()
  assert.deepEqual(n.calls, [
    ['ctx_popup_ready', { token: 5, width: 240, height: 400 }], ['ctx_popup_pick', { token: 5, path: [2, 0] }], ['ctx_popup_cancel', { token: 5 }], ['ctx_popup_loaded', undefined],
  ])
  const failing = popupTransport({ invoke: async () => { throw new Error('gone') }, listen: n.listen })
  await failing.pick(1, [0]); await failing.ready(1, { w: 1, h: 1 })
})

test('wiring: the shell installs the presenter and answers its frames; the menu window is an entry of the build, a command of the host, and is not closed by the window events', () => {
  const sidebar = src('../components/SidebarMenu.vue')
  assert.ok(sidebar.includes("installMenuPopup({ invoke, listen, backLabel: () => contextTexts(locale.value).backToMenu })"))
  assert.ok(sidebar.includes("popupMenu?.frameMessage(event, frame)"))
  assert.ok(sidebar.includes('<MContextProvider :locale="locale" :callbacks="ctxCallbacks" :capabilities="{ product: \'panel\', developer: developerMode.enabled.value }" />'), 'the window\'s own menu is still the Material menu')
  const vite = src('../../../vite.config.js')
  assert.ok(vite.includes("ctxMenu: resolve(rendererSrcPath, 'ctxmenu/index.html')"))
  const main = src('../ctxmenu/main.js'); assert.ok(main.includes("from 'foundation-ui/popup'") && main.includes("foundation-ui/all.css"))
  assert.ok(!main.includes("foundation-ui/vue'"), 'the menu window loads the menu, not the library')
  const rs = src('../../../src-tauri/src/main.rs')
  for (const c of ['ctx_popup_loaded', 'ctx_popup_open', 'ctx_popup_ready', 'ctx_popup_pick', 'ctx_popup_cancel', 'ctx_popup_selftest']) assert.ok(rs.includes(`ctx_popup::${c}`), c)
  assert.ok(rs.includes('mod menu_place;') && rs.includes('mod ctx_popup;'))
  const win = src('../../../src-tauri/src/windows.rs')
  assert.ok(win.includes('crate::ctx_popup::handle_window_event(window, event)') && win.includes('crate::ctx_popup::schedule_create('))
  const popup = src('../../../src-tauri/src/ctx_popup.rs')
  assert.ok(popup.includes('.transparent(true)') && popup.includes('.always_on_top(true)') && popup.includes('.skip_taskbar(true)') && popup.includes('.visible(false)') && popup.includes('.decorations(false)'))
  assert.ok(popup.includes('place_surface(') && popup.includes('monitor.work'), 'placed against the work area of the monitor')
})
