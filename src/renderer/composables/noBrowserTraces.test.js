import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync, readdirSync } from 'node:fs'
import vm from 'node:vm'

// SPEC fs52: nothing of the panel shows that a browser is under it. The guard (src-tauri/inject-traces.js) runs in every panel window and in every framed Web page.
const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8')
const guardSource = read('../../../src-tauri/inject-traces.js')

// a page of one element tree: `el(tag, attrs, parent)`; `closest` understands the selectors the guard uses
function el(tag, attrs = {}, parent = null) {
  const node = {
    nodeType: 1, tag, attrs, parentElement: parent,
    getAttribute: (name) => (name in attrs ? attrs[name] : null),
    hasAttribute: (name) => name in attrs,
    setAttribute: (name, value) => { attrs[name] = value },
    closest(selector) {
      for (let cursor = this; cursor; cursor = cursor.parentElement) {
        if (selector === '[draggable="true"]' && cursor.attrs.draggable === 'true') return cursor
        if (selector === 'a, img' && (cursor.tag === 'a' || cursor.tag === 'img')) return cursor
        if (selector.startsWith('input:not(') && (cursor.tag === 'textarea' || (cursor.tag === 'input' && !cursor.attrs.type) || cursor.attrs.contenteditable === 'true')) return cursor
        if (selector === 'a[href]' && cursor.tag === 'a' && cursor.attrs.href) return cursor
      }
      return null
    },
  }
  return node
}

function load(extra = {}) {
  const listeners = []
  const styles = []
  const doc = {
    head: { appendChild: (node) => styles.push(node) },
    documentElement: {},
    querySelector: () => null,
    createElement: () => ({ setAttribute() {}, textContent: '' }),
    addEventListener: (type, fn, opts) => listeners.push({ target: 'document', type, fn, opts }),
  }
  const win = { document: doc, addEventListener: (type, fn, opts) => listeners.push({ target: 'window', type, fn, opts }), getSelection: () => ({ isCollapsed: true, toString: () => '' }), ...extra }
  win.window = win
  vm.runInNewContext(guardSource, win)
  const on = (type, target = 'document') => listeners.find((l) => l.type === type && l.target === target)
  return { win, doc, listeners, styles, on }
}
const event = (props) => {
  const e = { defaultPrevented: false, preventDefault() { e.defaultPrevented = true }, ...props }
  return e
}

test('a link or an image cannot start a drag (its ghost shows the URL), the things that ask for a drag still can', () => {
  const { on } = load()
  const dragstart = on('dragstart').fn
  const page = el('div')
  const cases = [
    ['a link', el('a', { href: 'http://x/y' }, page), true],
    ['an image', el('img', {}, page), true],
    ['an image in a link', el('img', {}, el('a', { href: '/z' }, page)), true],
    ['a link inside a tile that can be dragged', el('a', { href: '/z' }, el('div', { draggable: 'true' }, page)), true],
    ['an image inside a tile that can be dragged', el('img', {}, el('div', { draggable: 'true' }, page)), true],
    ['a button', el('button', {}, page), true],
    ['a rail item (div)', el('div', { class: 'm-rail__item' }, page), true],
    ['a tile that can be rearranged', el('div', { draggable: 'true' }, page), false],
    ['something inside that tile', el('span', {}, el('div', { draggable: 'true' }, page)), false],
    ['an image that asked for a drag', el('img', { draggable: 'true' }, page), false],
    ['the selected text of a text field', el('textarea', {}, page), false],
  ]
  for (const [name, target, cancelled] of cases) {
    const e = event({ target })
    dragstart(e)
    assert.equal(e.defaultPrevented, cancelled, `${name}: ${cancelled ? 'is cancelled' : 'is left alone'}`)
  }
})

test('the browser keys are cancelled, our own keys are not, the DevTools keys stay in a developer build', () => {
  const { win } = load({ TAURI_PRODUCTION: true })
  const hit = win.__noBrowserTraces.browserKeyHit
  const key = (k, mods = {}) => ({ key: k, ctrlKey: false, shiftKey: false, altKey: false, metaKey: false, defaultPrevented: false, target: el('div'), ...mods })
  for (const e of [key('F5'), key('r', { ctrlKey: true }), key('R', { ctrlKey: true, shiftKey: true }), key('p', { ctrlKey: true }), key('f', { ctrlKey: true }), key('F3'), key('u', { ctrlKey: true }), key('s', { ctrlKey: true }),
    key('F12'), key('I', { ctrlKey: true, shiftKey: true }), key('J', { ctrlKey: true, shiftKey: true }), key('C', { ctrlKey: true, shiftKey: true }),
    key('+', { ctrlKey: true }), key('=', { ctrlKey: true }), key('-', { ctrlKey: true }), key('0', { ctrlKey: true }), key('ArrowLeft', { altKey: true }), key('ArrowRight', { altKey: true }), key('BrowserBack')]) {
    assert.equal(hit(e), true, `${e.ctrlKey ? 'Ctrl+' : ''}${e.altKey ? 'Alt+' : ''}${e.key} is a browser key`)
  }
  for (const e of [key('k', { ctrlKey: true }), key('c', { ctrlKey: true }), key('v', { ctrlKey: true }), key('x', { ctrlKey: true }), key('a', { ctrlKey: true }), key('z', { ctrlKey: true }), key('Tab'), key('Enter'), key('ArrowLeft'), key('a', { altKey: true, ctrlKey: true }),
    key('ArrowLeft', { altKey: true, defaultPrevented: true })]) {
    assert.equal(hit(e), false, `${e.ctrlKey ? 'Ctrl+' : ''}${e.altKey ? 'Alt+' : ''}${e.key} is left to the page`)
  }
  const dev = load({ TAURI_PRODUCTION: false }).win.__noBrowserTraces.browserKeyHit
  assert.equal(dev(key('F12')), false)
  assert.equal(dev(key('I', { ctrlKey: true, shiftKey: true })), false)
  assert.equal(dev(key('F5')), true, 'reload is a browser key in a developer build too')
})

test('the right-click menu is cancelled except in a text field or over a selection; a menu of the page is left alone', () => {
  const { on } = load()
  const menu = on('contextmenu').fn
  const page = el('div')
  for (const [name, target, cancelled] of [['a button', el('button', {}, page), true], ['a link', el('a', { href: '/x' }, page), true], ['plain page', page, true], ['a text field', el('input', {}, page), false], ['a textarea', el('textarea', {}, page), false]]) {
    const e = event({ target })
    menu(e)
    assert.equal(e.defaultPrevented, cancelled, name)
  }
  const mine = event({ target: el('input'), defaultPrevented: true })
  menu(mine)
  assert.equal(mine.defaultPrevented, true)
  const selected = load({ getSelection: () => ({ isCollapsed: false, toString: () => '192.168.1.5' }) }).on('contextmenu').fn
  const over = event({ target: el('span', {}, page) })
  selected(over)
  assert.equal(over.defaultPrevented, false, 'a selected value can be copied from the native menu (windows.rs keeps Copy only)')
})

test('a file dropped where nothing takes a drop is not opened by the engine; a place that takes the drop keeps it', () => {
  const { on } = load()
  const over = event({ dataTransfer: { dropEffect: 'copy' } })
  on('dragover').fn(over)
  assert.equal(over.defaultPrevented, true)
  assert.equal(over.dataTransfer.dropEffect, 'none', 'the cursor says no drop here')
  const drop = event({})
  on('drop').fn(drop)
  assert.equal(drop.defaultPrevented, true)
  const taken = event({ defaultPrevented: true, dataTransfer: { dropEffect: 'copy' } })
  on('dragover').fn(taken)
  assert.equal(taken.dataTransfer.dropEffect, 'copy', 'the theme editor and the file transfer accept their drops')
})

test('Ctrl+wheel and the pinch gestures cannot zoom the page', () => {
  const { on } = load()
  for (const e of [event({ ctrlKey: true }), event({ metaKey: true })]) { on('wheel', 'window').fn(e); assert.equal(e.defaultPrevented, true) }
  const plain = event({ ctrlKey: false, metaKey: false })
  on('wheel', 'window').fn(plain)
  assert.equal(plain.defaultPrevented, false, 'the plain wheel scrolls')
  assert.equal(on('wheel', 'window').opts.passive, false, 'a passive listener cannot cancel the wheel')
  for (const type of ['gesturestart', 'gesturechange', 'gestureend']) assert.ok(on(type), type)
})

test('a text field turns the spell checker and the saved values off when it takes the focus', () => {
  const { on } = load()
  const field = el('input')
  on('focusin').fn({ target: field })
  assert.equal(field.attrs.spellcheck, 'false')
  assert.equal(field.attrs.autocomplete, 'off')
  const keep = el('input', { autocomplete: 'one-time-code' })
  on('focusin').fn({ target: keep })
  assert.equal(keep.attrs.autocomplete, 'one-time-code', 'a field that names its autocomplete keeps it')
})

test('the style takes links and images out of the drag, keeps chrome from selecting and leaves values selectable', () => {
  const { win } = load()
  const css = win.__noBrowserTraces.CSS
  assert.match(css, /:where\(a, img, svg[^)]*\):not\(\[draggable="true"\]\) \{ -webkit-user-drag: none/)
  assert.match(css, /button[^{]*\.nav-link[^{]*\{ -webkit-user-select: none; user-select: none; \}/)
  assert.match(css, /pre, code[^{]*\{ -webkit-user-select: text; user-select: text; \}/)
  assert.ok(css.indexOf('user-select: none') < css.indexOf('user-select: text'), 'the values come last, so they win inside a chrome')
  assert.match(css, /tap-highlight-color: transparent/)
})

test('every window of the panel loads the guard, and the proxy puts it in front of the page script of the framed Web pages', () => {
  const entries = ['../sunshine-frame.js', '../desktop/main.js', '../toolbar/main.js', '../tool-window/main.js', '../home/main.js', '../latency-clock/main.js']
  for (const entry of entries) assert.match(read(entry), /^import '[./]*\/src-tauri\/inject-traces\.js'/, `${entry} imports the guard first`)
  const proxy = read('../../../src-tauri/src/proxy_server.rs')
  assert.ok(proxy.indexOf('result.push_str(INJECT_TRACES)') > 0 && proxy.indexOf('result.push_str(INJECT_TRACES)') < proxy.indexOf('result.push_str(INJECT_SCRIPT)'))
  assert.ok(proxy.indexOf('push_str(production_flag)') < proxy.indexOf('result.push_str(INJECT_TRACES)'), 'the guard reads TAURI_PRODUCTION, so the flag comes first')
  const script = read('../../../src-tauri/inject-script.js')
  assert.ok(!/addEventListener\('contextmenu', preventDefault/.test(script), 'the page script no longer cancels every right-click, text fields included')
  assert.ok(!/disableContextMenu/.test(script))
})

test('the engine closes what WebView2 shows of its browser', () => {
  const windows = read('../../../src-tauri/src/windows.rs')
  for (const call of ['SetIsStatusBarEnabled(false)', 'SetIsZoomControlEnabled(false)', 'SetIsBuiltInErrorPageEnabled(false)', 'SetAreBrowserAcceleratorKeysEnabled(cfg!(debug_assertions))', 'SetIsPinchZoomEnabled(false)', 'SetIsSwipeNavigationEnabled(false)', 'add_NewWindowRequested', 'add_DownloadStarting', 'add_ContextMenuRequested']) {
    assert.ok(windows.includes(call), call)
  }
  assert.ok(windows.includes('harden_browser_surface(&core_webview, &settings, &label)'), 'configure_webview_security applies it')
  assert.ok(/outside && web/.test(windows) && /starts_with\(own\.as_str\(\)\)/.test(windows), 'a new window opens the system browser for the pages of others, never for the proxy of the panel')
  const toolbar = read('../../../src-tauri/src/toolbar.rs')
  assert.ok(/windows::configure_webview_security\(&win\)/.test(toolbar), 'the pet window is hardened too')
  const builders = windows.match(/\.disable_drag_drop_handler\(\)/g) || []
  assert.ok(builders.length >= 2, 'the main and the desktop window keep Tauri\'s own drop handler off (HTML5 drag and drop works in them)')
})

test('no template of the panel puts a URL in a drag: a link or image with draggable="true" is a reviewed exception', () => {
  const dirs = ['../components', '../desktop/components', '../desktop/views', '../toolbar']
  const offenders = []
  const walk = (dir) => {
    for (const entry of readdirSync(new URL(dir + '/', import.meta.url), { withFileTypes: true })) {
      if (entry.isDirectory()) walk(`${dir}/${entry.name}`)
      else if (entry.name.endsWith('.vue')) {
        const source = read(`${dir}/${entry.name}`)
        for (const m of source.matchAll(/<(a|img)\b[^>]*\bdraggable="true"[^>]*>/g)) offenders.push(`${dir}/${entry.name}: ${m[0].slice(0, 60)}`)
      }
    }
  }
  for (const dir of dirs) walk(dir)
  assert.deepEqual(offenders, [])
})
