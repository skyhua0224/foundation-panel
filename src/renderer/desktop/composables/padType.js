import { computed, ref } from 'vue'
import { gamepadMapping, gamepadName } from './useGamepad.js'

/**
 * 手柄类型：按键提示（提示条、对话框、屏幕键盘、提示文字）按它显示手里那台手柄上真正印着的符号。
 *
 *   xbox      A B X Y  LB RB  LT RT
 *   ps        ✕ ○ □ △  L1 R1  L2 R2   （DualSense 与 DualShock 4 一样）
 *   nintendo  B A Y X  L R  ZL ZR      （位置同上，印的是另一套字母）
 *   generic   认不出的手柄：不假装它印着什么，用按键的位置（下/右/左/上）说话
 *
 * 动作的含义不变：确认永远是下方那颗面键（Xbox 的 A、PlayStation 的 ✕、Nintendo 的 B），返回永远是右边那颗——
 * 这里只决定它「印着什么」，不改行为（useGamepad 仍按标准映射的按钮号）。
 *
 * 类型来自两处，主机报告的优先：
 *   1. 串流时（Moonlight 连着这台主机），大屏看到的手柄是主机上的虚拟手柄，不是人手里的那台；
 *      主机在 /api/runtime/sessions 的 gamepad_type 里报告活跃的虚拟手柄（x360 / ds4 / ds5）。
 *   2. 否则看浏览器的 Gamepad API：id（含厂商号）与 mapping。
 */
export const PAD_TYPES = Object.freeze(['xbox', 'ps', 'nintendo', 'generic'])

// Sony 厂商号 054c 最可靠，必须锚定到 Vendor 字段（Product 恰为 054c 的杂牌手柄不是 Sony）；名称兜底覆盖改写 id 的第三方驱动。
// 蓝牙下 Sony 默认名以 "Wireless Controller" 开头，不能裸匹配该词组，否则 "Xbox Elite Wireless Controller" 也中招。
const PS_PATTERNS = [/\bvendor:\s*054c\b/i, /^054c-/i, /dualsense/i, /dualshock/i, /playstation/i, /^wireless controller/i, /ps[345] compatible/i]
// Nintendo 厂商号 057e；Pro Controller / Joy-Con 的名字兜底
const NINTENDO_PATTERNS = [/\bvendor:\s*057e\b/i, /^057e-/i, /nintendo/i, /switch pro/i, /joy-?con/i, /^pro controller/i]
// Xbox 排布：Microsoft 045e、XInput 系；Valve（Steam Controller / Steam Deck，28de）和 Stadia（18d1）的 A B X Y 也在 Xbox 的位置上
const XBOX_PATTERNS = [/\bvendor:\s*(045e|28de|18d1)\b/i, /^(045e|28de|18d1)-/i, /xbox/i, /xinput/i, /x360/i, /steam (controller|deck)/i, /stadia/i]

/**
 * 从 Gamepad API 的 id 与 mapping 判断类型。纯函数，用真实设备的 id 字符串做单元测试。
 * 认得出的品牌看 id（厂商号、名字）。认不出的看 mapping：浏览器说它是 'standard'（按钮号就是标准位置，
 * 这种手柄印的几乎都是 A B X Y）按 Xbox；mapping 不是标准的，按钮号与位置的对应不可信，按 generic。
 * 没有任何信息（还没见到手柄）按 Xbox：标准映射的按钮号本来就是 Xbox 的约定。
 * @param {string} id gamepad.id
 * @param {string} [mapping] gamepad.mapping；没给时从 id 里的 "STANDARD GAMEPAD"（Chromium 的写法）推断
 */
export function detectPadType(id, mapping) {
  const name = String(id || '').trim()
  if (!name) return 'xbox'
  if (PS_PATTERNS.some((pattern) => pattern.test(name))) return 'ps'
  if (NINTENDO_PATTERNS.some((pattern) => pattern.test(name))) return 'nintendo'
  if (XBOX_PATTERNS.some((pattern) => pattern.test(name))) return 'xbox'
  const standard = mapping === 'standard' || (mapping === undefined && /standard gamepad/i.test(name))
  return standard ? 'xbox' : 'generic'
}

/** 主机报告的虚拟手柄类型 → 提示类型；主机没报告（空串、未知值）返回 null。 */
export function padTypeFromHost(value) {
  switch (String(value || '').toLowerCase()) {
    case 'x360': return 'xbox'
    case 'ds4':
    case 'ds5': return 'ps'
    default: return null
  }
}

/** /api/runtime/sessions 的回答里，活跃的虚拟手柄类型（第一个有报告的会话）。回答不是预期的形状时返回 null。 */
export function hostPadTypeOf(answer) {
  const sessions = Array.isArray(answer?.sessions) ? answer.sessions : []
  for (const session of sessions) {
    const type = padTypeFromHost(session?.gamepad_type)
    if (type) return type
  }
  return null
}

/** 主机报告的类型（串流中）。null = 没有串流或主机没报告，此时看本机的手柄。 */
export const hostPadType = ref(null)

export function setHostPadType(type) {
  hostPadType.value = PAD_TYPES.includes(type) ? type : null
}

/** 当前的手柄类型；手柄换了、串流开始或结束都即时跟着变。 */
export const padType = computed(() => hostPadType.value || detectPadType(gamepadName.value, gamepadMapping.value))
