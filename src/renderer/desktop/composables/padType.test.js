import test from 'node:test'
import assert from 'node:assert/strict'
import { PAD_TYPES, detectPadType, hostPadTypeOf, padType, padTypeFromHost, setHostPadType } from './padType.js'
import { gamepadMapping, gamepadName } from './useGamepad.js'

// 真实设备在 Windows/Chromium 上报告的 gamepad.id（含 mapping）
const PADS = [
  ['DualSense Wireless Controller (STANDARD GAMEPAD Vendor: 054c Product: 0ce6)', 'standard', 'ps'],
  ['Wireless Controller (STANDARD GAMEPAD Vendor: 054c Product: 09cc)', 'standard', 'ps'], // DualShock 4 蓝牙
  ['PLAYSTATION(R)3 Controller (STANDARD GAMEPAD Vendor: 054c Product: 0268)', 'standard', 'ps'],
  ['054c-0ce6-DualSense Wireless Controller', '', 'ps'], // Firefox 的写法
  ['Xbox 360 Controller (XInput STANDARD GAMEPAD Vendor: 045e Product: 028e)', 'standard', 'xbox'],
  ['Xbox One Controller (STANDARD GAMEPAD Vendor: 045e Product: 02ea)', 'standard', 'xbox'],
  ['Xbox Elite Wireless Controller (STANDARD GAMEPAD Vendor: 045e Product: 0b00)', 'standard', 'xbox'],
  ['Steam Deck Controller (STANDARD GAMEPAD Vendor: 28de Product: 1205)', 'standard', 'xbox'],
  ['Pro Controller (STANDARD GAMEPAD Vendor: 057e Product: 2009)', 'standard', 'nintendo'],
  ['Joy-Con (L+R) (STANDARD GAMEPAD Vendor: 057e Product: 200e)', 'standard', 'nintendo'],
  ['Generic USB Joystick (STANDARD GAMEPAD Vendor: 0079 Product: 0006)', 'standard', 'xbox'], // 认不出的标准映射：按标准映射的 Xbox 约定
  ['USB Gamepad (Vendor: 0810 Product: e501)', '', 'generic'], // 认不出且非标准映射
  ['8BitDo Pro 2 (Vendor: 2dc8 Product: 6003)', '', 'generic'],
]

test('每种手柄识别为它的类型（DualSense 与 DS4 都是 ps）', () => {
  for (const [id, mapping, want] of PADS) assert.equal(detectPadType(id, mapping), want, id)
})

test('mapping 没给时从 id 里的 STANDARD GAMEPAD 推断', () => {
  assert.equal(detectPadType('Generic USB Joystick (STANDARD GAMEPAD Vendor: 0079 Product: 0006)'), 'xbox')
  assert.equal(detectPadType('USB Gamepad (Vendor: 0810 Product: e501)'), 'generic')
})

test('没有手柄信息按 xbox；Product 号恰为 054c 的杂牌不是 Sony；Xbox 的 Wireless Controller 不是 PlayStation', () => {
  for (const id of ['', null, undefined]) assert.equal(detectPadType(id), 'xbox')
  assert.equal(detectPadType('Generic Pad (STANDARD GAMEPAD Vendor: 0079 Product: 054c)', 'standard'), 'xbox')
  assert.equal(detectPadType('Xbox Elite Wireless Controller (STANDARD GAMEPAD Vendor: 045e)', 'standard'), 'xbox')
  assert.equal(detectPadType('Wireless Controller (STANDARD GAMEPAD Vendor: 054c Product: 09cc)', 'standard'), 'ps')
})

test('主机报告的虚拟手柄：x360 是 xbox，ds4 与 ds5 都是 ps，空与未知不算', () => {
  assert.equal(padTypeFromHost('x360'), 'xbox')
  assert.equal(padTypeFromHost('ds4'), 'ps')
  assert.equal(padTypeFromHost('ds5'), 'ps')
  for (const v of ['', null, undefined, 'xone', 'auto']) assert.equal(padTypeFromHost(v), null, String(v))
})

test('会话回答里取活跃的虚拟手柄类型：第一个有报告的会话，形状不对返回 null', () => {
  assert.equal(hostPadTypeOf({ sessions: [{ client_name: 'a', gamepad_type: '' }, { client_name: 'b', gamepad_type: 'ds5' }] }), 'ps')
  assert.equal(hostPadTypeOf({ sessions: [{ gamepad_type: 'x360' }] }), 'xbox')
  assert.equal(hostPadTypeOf({ sessions: [] }), null)
  for (const bad of [null, undefined, {}, [], { sessions: 'x' }, { sessions: [null, 3] }]) assert.equal(hostPadTypeOf(bad), null)
})

test('当前类型：主机报告的优先，没有就看本机手柄，手柄换了即时跟着变', () => {
  setHostPadType(null)
  gamepadName.value = 'Xbox One Controller (STANDARD GAMEPAD Vendor: 045e Product: 02ea)'; gamepadMapping.value = 'standard'
  assert.equal(padType.value, 'xbox')
  gamepadName.value = 'DualSense Wireless Controller (STANDARD GAMEPAD Vendor: 054c Product: 0ce6)'
  assert.equal(padType.value, 'ps')
  gamepadName.value = 'Pro Controller (STANDARD GAMEPAD Vendor: 057e Product: 2009)'
  assert.equal(padType.value, 'nintendo')
  // 串流：大屏看到的是主机的虚拟 x360，主机说活跃的是 ds5
  setHostPadType(padTypeFromHost('ds5'))
  gamepadName.value = 'Xbox 360 Controller (XInput STANDARD GAMEPAD Vendor: 045e Product: 028e)'
  assert.equal(padType.value, 'ps')
  setHostPadType(null)
  assert.equal(padType.value, 'xbox')
  setHostPadType('no-such-type')
  assert.equal(padType.value, 'xbox')
  gamepadName.value = ''; gamepadMapping.value = ''
  assert.deepEqual([...PAD_TYPES], ['xbox', 'ps', 'nintendo', 'generic'])
})
