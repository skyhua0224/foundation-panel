import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { PAD_ACTION_BUTTONS, fillHint, glyphOf, glyphsFor, splitHint, wordsFor } from './padGlyphs.js'
import { en } from '../i18n/en.js'
import { zh } from '../i18n/zh.js'

const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8')

test('确认与返回在每类手柄上都在同一个物理位置：下方与右边那颗面键，只是印的不同', () => {
  const confirm = { xbox: 'A', ps: '✕', nintendo: 'B', generic: '↓' }
  const back = { xbox: 'B', ps: '○', nintendo: 'A', generic: '→' }
  for (const type of ['xbox', 'ps', 'nintendo', 'generic']) {
    assert.equal(wordsFor('confirm', type), confirm[type], `confirm ${type}`)
    assert.equal(wordsFor('back', type), back[type], `back ${type}`)
    assert.equal(glyphsFor('confirm', type)[0].position, 'south')
    assert.equal(glyphsFor('back', type)[0].position, 'east')
  }
  // 行为没动：动作仍绑在标准映射的 A 与 B
  assert.deepEqual([PAD_ACTION_BUTTONS.confirm, PAD_ACTION_BUTTONS.back, PAD_ACTION_BUTTONS.favorite, PAD_ACTION_BUTTONS.search], [['A'], ['B'], ['X'], ['Y']])
})

test('Xbox：A B X Y 与 LB RB / LT RT；PlayStation：✕ ○ □ △ 与 L1 R1 / L2 R2（DualSense 与 DS4 一样）；Nintendo：B A Y X 与 L R / ZL ZR', () => {
  assert.deepEqual(['confirm', 'back', 'favorite', 'search', 'pages', 'scroll'].map((a) => wordsFor(a, 'xbox')), ['A', 'B', 'X', 'Y', 'LB/RB', 'LT/RT'])
  assert.deepEqual(['confirm', 'back', 'favorite', 'search', 'pages', 'scroll'].map((a) => wordsFor(a, 'ps')), ['✕', '○', '□', '△', 'L1/R1', 'L2/R2'])
  assert.deepEqual(['confirm', 'back', 'favorite', 'search', 'pages', 'scroll'].map((a) => wordsFor(a, 'nintendo')), ['B', 'A', 'Y', 'X', 'L/R', 'ZL/ZR'])
  assert.deepEqual(['pages', 'scroll'].map((a) => wordsFor(a, 'generic')), ['L1/R1', 'L2/R2'])
  assert.equal(wordsFor('menu', 'ps'), 'Options'); assert.equal(wordsFor('menu', 'nintendo'), '+'); assert.equal(wordsFor('menu', 'xbox'), 'Menu')
})

test('PlayStation 的面键是我们画的形状，认不出的手柄画位置，Xbox 与 Nintendo 画字', () => {
  assert.deepEqual(['A', 'B', 'X', 'Y'].map((b) => glyphOf(b, 'ps').shape), ['cross', 'circle', 'square', 'triangle'])
  assert.deepEqual(['A', 'B', 'X', 'Y'].map((b) => glyphOf(b, 'generic').shape), ['south', 'east', 'west', 'north'])
  for (const type of ['xbox', 'nintendo']) for (const b of ['A', 'B', 'X', 'Y']) assert.equal(glyphOf(b, type).shape, null)
  assert.deepEqual(['A', 'B', 'X', 'Y'].map((b) => glyphOf(b, 'xbox').tone), ['green', 'red', 'blue', 'yellow'])
  assert.deepEqual(['A', 'B', 'X', 'Y'].map((b) => glyphOf(b, 'ps').tone), ['blue', 'red', 'pink', 'green'])
  assert.deepEqual(['LB', 'LT', 'L3', 'Start'].map((b) => glyphOf(b, 'ps').kind), ['bumper', 'trigger', 'stick', 'menu'])
})

test('一句话里的 {action} 换成这类手柄上的字；不认识的记号原样留着', () => {
  const sentence = 'D-pad to move · {confirm} to type · {back} to cancel · {nothing}'
  assert.equal(fillHint(sentence, 'xbox'), 'D-pad to move · A to type · B to cancel · {nothing}')
  assert.equal(fillHint(sentence, 'ps'), 'D-pad to move · ✕ to type · ○ to cancel · {nothing}')
  assert.equal(fillHint(sentence, 'nintendo'), 'D-pad to move · B to type · A to cancel · {nothing}')
  assert.deepEqual(splitHint('a {confirm} b {back}{x}'), [{ text: 'a ' }, { action: 'confirm' }, { text: ' b ' }, { action: 'back' }, { text: '{x}' }])
  assert.deepEqual(splitHint(''), [])
  assert.equal(wordsFor('no-such-action', 'ps'), ''); assert.deepEqual(glyphsFor('no-such-action'), [])
})

test('每句带按键的话都走这一个映射：文案里没有写死的键名，记号都认得', () => {
  for (const [name, t] of [['en', en], ['zh', zh]]) {
    for (const text of [t.osk.hint, t.settings.oskAutoOpenDesc, t.settings.gamepadCursorDesc]) {
      assert.doesNotMatch(text, /(^|[^A-Za-z])(A|B|X|Y|L3|LB|RB)([^A-Za-z]|$)/, `${name}: ${text}`)
      for (const [, action] of text.matchAll(/\{([a-z]+)\}/g)) assert.ok(PAD_ACTION_BUTTONS[action], `${name}: {${action}}`)
    }
  }
  assert.equal(splitHint(en.osk.hint).filter((p) => p.action).length, 2)
})

test('提示条、屏幕键盘、对话框、设置说明都用 PadGlyph / PadHint / fillHint，不再各自拼键名', () => {
  const legend = read('../components/GamepadLegend.vue')
  assert.match(legend, /<PadGlyph :action="hint\.action"/)
  assert.doesNotMatch(legend, /chipFor|hint\.glyph/)
  assert.match(read('../components/OnScreenKeyboard.vue'), /<PadHint :text="t\.osk\.hint" \/>/)
  assert.match(read('../components/OnScreenKeyboard.vue'), /<PadGlyph v-if="gamepadActive" action="back"/)
  assert.match(read('../components/ConfirmDialog.vue'), /<PadGlyph v-if="gamepadActive" action="back"/)
  assert.match(read('../../components/m3e/PanelOptions.vue'), /fillHint\(st\.value\.oskAutoOpenDesc, padType\.value\)/)
})
