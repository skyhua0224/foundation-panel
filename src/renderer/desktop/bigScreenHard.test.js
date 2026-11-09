import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import test from 'node:test'
import { bindGamepadConnectionEvents, NATIVE_GAMEPAD_EVENTS } from './composables/useGamepad.js'
import { isFocusVisible } from './composables/useFocusNav.js'

const root = fileURLToPath(new URL('./', import.meta.url))
const read = (file) => readFileSync(join(root, file), 'utf8')

test('晚接手柄事件会立即触发重扫：模拟 browser event 后连接回调只需一次', () => {
  const target = new EventTarget()
  const calls = []
  const stop = bindGamepadConnectionEvents(target, {
    onConnected: () => calls.push('connected'),
    onDisconnected: () => calls.push('disconnected'),
  })

  target.dispatchEvent(new Event('gamepadconnected'))
  target.dispatchEvent(new Event('gamepaddisconnected'))
  stop()
  target.dispatchEvent(new Event('gamepadconnected'))

  assert.deepEqual(calls, ['connected', 'disconnected'])
  assert.ok(NATIVE_GAMEPAD_EVENTS.includes('gamepad-connected'))
  assert.ok(NATIVE_GAMEPAD_EVENTS.includes('native-gamepad-connected'))
  assert.match(read('composables/useGamepad.js'), /startPolling\(POLL_INTERVAL_MS\)\s*\n\s*poll\(\)/)
})

test('焦点恢复拒绝隐藏或禁用目标，并在页面切换与连接状态变化时运行', () => {
  const previousWindow = globalThis.window
  globalThis.window = { getComputedStyle: () => ({ visibility: 'visible', display: 'block' }) }
  const element = (attrs = {}, hiddenAncestor = null) => ({
    isConnected: true,
    hasAttribute: (name) => name in attrs && attrs[name] !== false,
    getAttribute: (name) => attrs[name] ?? null,
    closest: () => hiddenAncestor,
    getBoundingClientRect: () => ({ width: 10, height: 10 }),
  })
  assert.equal(isFocusVisible(element()), true)
  assert.equal(isFocusVisible(element({ 'aria-disabled': 'true' })), false)
  assert.equal(isFocusVisible(element({}, {})), false)
  globalThis.window = previousWindow

  const app = read('DesktopApp.vue')
  assert.match(app, /watch\(\[activeNav, gamepadConnected\]/)
  assert.match(read('composables/useFocusNav.js'), /else focusFirst\(activeScopeRoot\(\)\)/)
})

test('B、Circle、Escape 共用逐层返回，顶层退出通过可聚焦取消按钮确认', () => {
  const app = read('DesktopApp.vue')
  const dialog = read('components/ConfirmDialog.vue')
  assert.match(app, /keyboardBackListener = handleKeyboardBack/)
  assert.match(app, /if \(event\.key !== 'Escape'/)
  assert.match(app, /await requestExitShell\(\)/)
  assert.match(app, /requestConfirm\(\{[\s\S]*exitTitle/)
  assert.match(dialog, /data-focus-key="confirm-cancel"/)
})

test('手柄鼠标模式显示稳定状态，并继续让光标模式绕开空间导航', () => {
  const app = read('DesktopApp.vue')
  const legend = read('components/GamepadLegend.vue')
  assert.match(legend, /data-cursor-mode/) 
  assert.match(app, /setCursorEnabled\(!cursorEnabled\.value\)/)
  assert.match(app, /isCursorMode: \(\) => cursorEnabled\.value/)
})
