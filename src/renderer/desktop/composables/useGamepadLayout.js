import { computed } from 'vue'
import { detectPadType, padType } from './padType.js'
import { glyphsFor } from './padGlyphs.js'

/**
 * 手柄布局识别：同一套「标准映射」按键在不同手柄上印着不同的符号，
 * 提示必须跟着用户手里那台走——拿 DualSense 的人看到 "A 确定" 会按错。
 *
 * 识别与映射现在只在两个地方：padType.js（手柄是什么：Xbox / PlayStation / Nintendo / 认不出，
 * 串流时主机报告的虚拟手柄优先）和 padGlyphs.js（动作在这类手柄上画什么）。这里留着原来的两个入口，
 * 返回同样的东西：`detectLayout(id)` 给 id 的类型，`chipFor(action)` 给动作的字与色调（文字版）。
 */

/**
 * 从 gamepad.id 判断类型。纯函数。
 * @returns {'xbox' | 'ps' | 'nintendo' | 'generic'} 没有信息按 xbox（标准映射的按钮号即 Xbox 约定）
 */
export function detectLayout(idString) {
  return detectPadType(idString)
}

/** 当前手柄的类型（含串流时主机报告的）；随手柄切换实时更新。 */
export const gamepadLayout = computed(() => padType.value)

/**
 * 取某个动作在指定类型下的按键字与色调（文字版）。
 * @returns {{ glyph: string, tone: string }}
 */
export function chipFor(actionId, layout = gamepadLayout.value) {
  const glyphs = glyphsFor(actionId, layout)
  if (actionId === 'menu') return { glyph: '☰', tone: 'neutral' }
  return { glyph: glyphs.map((g) => g.text).join('/'), tone: glyphs[0]?.tone || 'neutral' }
}
