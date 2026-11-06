import { controllerButtonChip } from '../../composables/controllerButtons.js'

/**
 * 按键提示的唯一映射：动作 → 手柄上的哪几颗键 → 这类手柄上印着什么（字、形状、颜色）。
 * 提示条、对话框、屏幕键盘、提示文字都从这里取，换手柄类型（padType.js）时全部一起变。
 *
 * 键用标准映射的名字：A 是下方那颗面键，B 右、X 左、Y 上；LB RB 肩键，LT RT 扳机，L3 R3 摇杆键，Start 菜单键，Back 视图键。
 * 动作与键的对应跟 useGamepad.js 的 BUTTON_ACTIONS 一致：这里只管「怎么显示」，不改「按哪颗键做什么」。
 */
export const PAD_ACTION_BUTTONS = Object.freeze({
  confirm: ['A'], back: ['B'], favorite: ['X'], search: ['Y'], menu: ['Start'],
  pages: ['LB', 'RB'], scroll: ['LT', 'RT'], cursor: ['L3'], home: ['R3'],
  click: ['A'], exitCursor: ['B'],
  // 屏幕键盘：A 输入当前键，B 取消
  type: ['A'], cancel: ['B'],
})

const FACE = new Set(['A', 'B', 'X', 'Y'])
const POSITION = { A: 'south', B: 'east', X: 'west', Y: 'north' }

// 颜色是功能色（tonal 容器），不是任何品牌的图：Xbox 的 A B X Y 与 PlayStation 的 ✕ ○ □ △ 各有一组按位置对应的色位；
// Nintendo 的键是同一种颜色，用中性色；认不出的手柄也是中性色，位置靠形状说话。
const FACE_TONES = {
  xbox: { A: 'green', B: 'red', X: 'blue', Y: 'yellow' },
  ps: { A: 'blue', B: 'red', X: 'pink', Y: 'green' },
  nintendo: { A: 'neutral', B: 'neutral', X: 'neutral', Y: 'neutral' },
  generic: { A: 'neutral', B: 'neutral', X: 'neutral', Y: 'neutral' },
}
const PS_SHAPES = { A: 'cross', B: 'circle', X: 'square', Y: 'triangle' }

/**
 * 一颗键在某类手柄上的样子。
 * kind: face（面键）| bumper（肩键）| trigger（扳机）| stick（摇杆键）| menu（菜单键）| view（视图键）
 * shape: PlayStation 的 cross circle square triangle；generic 面键的位置 south east west north；其余 null（字就是符号）
 */
export function glyphOf(button, type = 'xbox') {
  const layout = FACE_TONES[type] ? type : 'xbox'
  const chip = controllerButtonChip(button, layout)
  if (FACE.has(button)) {
    const shape = layout === 'ps' ? PS_SHAPES[button] : layout === 'generic' ? POSITION[button] : null
    return { button, kind: 'face', text: chip.glyph, shape, tone: FACE_TONES[layout][button], position: POSITION[button] }
  }
  const kind = button === 'LB' || button === 'RB' ? 'bumper' : button === 'LT' || button === 'RT' ? 'trigger' : button === 'Start' ? 'menu' : button === 'Back' ? 'view' : 'stick'
  return { button, kind, text: chip.glyph, shape: null, tone: 'neutral', position: '' }
}

/** 一个动作的键（按序）：[{ button, kind, text, shape, tone, position }]；不认识的动作返回 []。 */
export function glyphsFor(action, type = 'xbox') {
  return (PAD_ACTION_BUTTONS[action] || []).map((button) => glyphOf(button, type))
}

/** 动作的键写成字：'A'、'✕'、'L1/R1'；用在 title 与 aria-label 这类只能放字的地方。不认识的动作返回 ''。 */
export function wordsFor(action, type = 'xbox') {
  return glyphsFor(action, type).map((glyph) => glyph.text).join('/')
}

const TOKEN = /\{([a-zA-Z]+)\}/g

/** 把句子里的 {confirm} {back} …换成这类手柄上的字（纯文字版，给 title / aria-label）。不认识的 {x} 原样留着。 */
export function fillHint(template, type = 'xbox') {
  return String(template ?? '').replace(TOKEN, (whole, action) => wordsFor(action, type) || whole)
}

/** 句子切成片段：[{ text }] 与 [{ action }]，给要把按键画成图标的地方（PadHint.vue）。不认识的 {x} 留在文字里。 */
export function splitHint(template) {
  const parts = []
  let last = 0
  const source = String(template ?? '')
  for (const match of source.matchAll(TOKEN)) {
    if (!PAD_ACTION_BUTTONS[match[1]]) continue
    if (match.index > last) parts.push({ text: source.slice(last, match.index) })
    parts.push({ action: match[1] })
    last = match.index + match[0].length
  }
  if (last < source.length) parts.push({ text: source.slice(last) })
  return parts
}
