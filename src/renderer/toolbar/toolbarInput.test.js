import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import vm from 'node:vm'

// Execute the component's actual pointer and native-window handlers with a
// controllable Tauri window. The native window itself is tested on Windows.
const component = readFileSync(new URL('./ToolbarApp.vue', import.meta.url), 'utf8')
const section = (start, end) => {
  const from = component.indexOf(start)
  const to = component.indexOf(end, from)
  assert.ok(from >= 0 && to > from, `component section ${start}`)
  return component.slice(from, to)
}

const setup = ({ setIgnore = async () => {}, invoke = async () => {}, maxTouchPoints = 0 } = {}) => {
  const listeners = new Map()
  const timers = new Map()
  let timerId = 0
  let now = 1000
  const menuVisible = { value: false }
  const speechVisible = { value: false }
  const windowApi = {
    setIgnoreCursorEvents: setIgnore,
    outerPosition: async () => ({ x: 0, y: 0 }),
    outerSize: async () => ({ width: 240, height: 240 }),
    scaleFactor: async () => 1,
    setPosition: async () => {},
    onScaleChanged: async () => () => {},
    startDragging: async () => {},
  }
  const context = vm.createContext({
    menuVisible, speechVisible,
    visionAbortController: null,
    cancelSpeechAnimationFrame() {},
    speechTimer: null,
    componentDisposed: false,
    getCurrentWindow: () => windowApi,
    cursorPosition: async () => ({ x: 500, y: 500 }),
    PhysicalPosition: class { constructor(x, y) { this.x = x; this.y = y } },
    invoke,
    watch() {},
    document: {
      addEventListener: (type, fn) => listeners.set(type, fn),
      removeEventListener: (type, fn) => { if (listeners.get(type) === fn) listeners.delete(type) },
    },
    window: { devicePixelRatio: 1 },
    navigator: { maxTouchPoints },
    setTimeout: (fn, delay) => { const id = ++timerId; timers.set(id, { fn, delay }); return id },
    clearTimeout: (id) => timers.delete(id),
    requestAnimationFrame: (fn) => { const id = ++timerId; timers.set(id, { fn, delay: 16 }); return id },
    cancelAnimationFrame: (id) => timers.delete(id),
    Date: { now: () => now },
    console,
  })
  const code = section('let longPressTimer = null', 'const handleOutsideClick =') +
    section('const appWindow = getCurrentWindow()', 'onMounted(async () =>') + `
      ({ onDragStart, onDragMove, onDragEnd, onIconClick, onIconContextMenu,
         setCursorIgnore, stopCursorHitTest, hitTestTick, applyToolbarWindowMode,
         enableHitTest: () => { hitTestEnabled = true; return ++hitTestGeneration },
         state: () => ({ ignore: cursorIgnoreApplied, desired: cursorIgnoreDesired }) })`
  const handlers = vm.runInContext(code, context)
  const target = {
    classList: { contains: (name) => name === 'toolbar-icon' },
    setPointerCapture() {}, hasPointerCapture: () => true, releasePointerCapture() {},
  }
  const pointer = (type, extra = {}) => ({
    type, pointerType: 'touch', isPrimary: true, pointerId: 1, button: 0,
    clientX: 10, clientY: 10, screenX: 10, screenY: 10, currentTarget: target,
    preventDefault() {}, ...extra,
  })
  return {
    ...handlers, menuVisible, speechVisible, timers, listeners, pointer,
    advance(ms) { now += ms },
    fire(delay) {
      const entry = [...timers].find(([, timer]) => timer.delay === delay)
      assert.ok(entry, `timer ${delay}`)
      timers.delete(entry[0])
      entry[1].fn()
    },
  }
}

test('touch tap opens once even when WebView also generates a click', async () => {
  const h = setup()
  h.onDragStart(h.pointer('pointerdown'))
  await h.onDragEnd(h.pointer('pointerup'))
  assert.equal(h.menuVisible.value, true)
  h.onIconClick()
  assert.equal(h.menuVisible.value, true)
  h.advance(701)
  h.onIconClick()
  assert.equal(h.menuVisible.value, false)
})

test('long press and right click open the menu without double toggling', async () => {
  const h = setup()
  h.onDragStart(h.pointer('pointerdown'))
  h.fire(550)
  assert.equal(h.menuVisible.value, true)
  h.onIconContextMenu({ pointerType: 'touch' })
  await h.onDragEnd(h.pointer('pointerup'))
  h.onIconClick()
  assert.equal(h.menuVisible.value, true)
  h.advance(701)
  h.onIconContextMenu({ pointerType: 'mouse' })
  assert.equal(h.menuVisible.value, false)
})

test('a second finger cannot cancel the primary long press', async () => {
  const h = setup()
  h.onDragStart(h.pointer('pointerdown'))
  await h.onDragEnd(h.pointer('pointerup', { pointerId: 2 }))
  h.fire(550)
  assert.equal(h.menuVisible.value, true)
  await h.onDragEnd(h.pointer('pointerup'))
})

test('moving or cancelling a touch does not open the menu', async () => {
  const h = setup()
  h.onDragStart(h.pointer('pointerdown'))
  h.onDragMove(h.pointer('pointermove', { clientX: 30 }))
  assert.equal([...h.timers.values()].some((timer) => timer.delay === 550), false)
  await h.onDragEnd(h.pointer('pointerup', { clientX: 30 }))
  assert.equal(h.menuVisible.value, false)
  h.advance(701)
  h.onDragStart(h.pointer('pointerdown'))
  await h.onDragEnd(h.pointer('pointercancel'))
  assert.equal(h.menuVisible.value, false)
})

test('a real mouse right click still works immediately after a touch', async () => {
  const h = setup()
  h.onDragStart(h.pointer('pointerdown'))
  await h.onDragEnd(h.pointer('pointerup'))
  assert.equal(h.menuVisible.value, true)
  h.onIconContextMenu({ pointerType: 'mouse' })
  assert.equal(h.menuVisible.value, false)
  h.onIconContextMenu({ button: 2 })
  assert.equal(h.menuVisible.value, true, 'a MouseEvent may have no pointerType or sourceCapabilities')
  h.onIconContextMenu({ button: 2, sourceCapabilities: { firesTouchEvents: true } })
  assert.equal(h.menuVisible.value, true, 'a touch-generated context menu must not toggle twice')
})

test('a real mouse click without pointer metadata still works immediately after a touch', async () => {
  const h = setup()
  h.onDragStart(h.pointer('pointerdown'))
  await h.onDragEnd(h.pointer('pointerup'))
  assert.equal(h.menuVisible.value, true)
  h.onDragStart(h.pointer('pointerdown', { pointerType: 'mouse' }))
  await h.onDragEnd(h.pointer('pointerup', { pointerType: 'mouse' }))
  h.onIconClick({ button: 0 })
  assert.equal(h.menuVisible.value, false)
})

test('mouse drag uses the native move loop without opening the menu', async () => {
  const calls = []
  const h = setup({ invoke: async (command) => { calls.push(command); return false } })
  h.onDragStart(h.pointer('pointerdown', { pointerType: 'mouse' }))
  h.onDragMove(h.pointer('pointermove', { pointerType: 'mouse', screenX: 30 }))
  await new Promise((resolve) => setImmediate(resolve))
  h.onIconClick({ pointerType: 'mouse' })
  assert.equal(h.menuVisible.value, false)
  assert.ok(calls.includes('is_primary_mouse_button_pressed'))
  assert.ok(calls.includes('save_toolbar_position'))
})

test('closing a speech bubble restores native mouse and touch hit testing', async () => {
  const calls = []
  const h = setup({ setIgnore: async (value) => calls.push(value) })
  h.enableHitTest()
  h.setCursorIgnore(true)
  await new Promise((resolve) => setImmediate(resolve))
  assert.equal(h.state().ignore, true)
  h.stopCursorHitTest()
  await new Promise((resolve) => setImmediate(resolve))
  assert.deepEqual(calls, [true, false])
  assert.equal(h.state().ignore, false)
})

test('stopping while click-through is in flight restores the window afterward', async () => {
  const calls = []
  let finishIgnore
  const h = setup({ setIgnore: (value) => {
    calls.push(value)
    return value ? new Promise((resolve) => { finishIgnore = resolve }) : Promise.resolve()
  } })
  h.enableHitTest()
  h.setCursorIgnore(true)
  h.stopCursorHitTest()
  finishIgnore()
  await new Promise((resolve) => setImmediate(resolve))
  assert.deepEqual(calls, [true, false])
  assert.equal(h.state().ignore, false)
})

test('speech stays touchable when a touch device is available', async () => {
  const calls = []
  const h = setup({ maxTouchPoints: 1, invoke: async (command, args) => calls.push([command, args.expanded]) })
  h.speechVisible.value = true
  await h.applyToolbarWindowMode()
  assert.deepEqual(calls, [['set_toolbar_window_expanded', true]])
  assert.equal([...h.timers.values()].some((timer) => timer.delay === 0), false)
  assert.equal(h.state().desired, false)
})

test('rapid window-mode changes settle on the latest requested size', async () => {
  const calls = []
  let finishExpand
  const h = setup({ invoke: (command, args) => {
    calls.push(args.expanded)
    return args.expanded ? new Promise((resolve) => { finishExpand = resolve }) : Promise.resolve()
  } })
  h.speechVisible.value = true
  const expanded = h.applyToolbarWindowMode()
  await Promise.resolve()
  h.speechVisible.value = false
  const compact = h.applyToolbarWindowMode()
  finishExpand()
  await Promise.all([expanded, compact])
  assert.deepEqual(calls, [true, false])
  assert.equal([...h.timers.values()].some((timer) => timer.delay === 0), false)
})

test('an old cursor sample cannot re-enable click-through after the mode stops', async () => {
  const calls = []
  const h = setup({ setIgnore: async (value) => calls.push(value) })
  const generation = h.enableHitTest()
  const tick = h.hitTestTick(generation)
  h.stopCursorHitTest()
  await tick
  assert.deepEqual(calls, [])
  assert.equal(h.state().desired, false)
})
