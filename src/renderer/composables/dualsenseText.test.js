import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dualSenseErrorCode, dualSenseErrorText, formatText, refusedSaveText } from './dualsenseText.js'
import { zh } from '../desktop/i18n/zh.js'
import { en } from '../desktop/i18n/en.js'

test('placeholders are filled and unknown ones stay visible', () => {
  assert.equal(formatText('已保存 {count} 项', { count: 3 }), '已保存 3 项')
  assert.equal(formatText('{a} 和 {b}', { a: 1 }), '1 和 {b}')
  assert.equal(formatText(undefined), '')
})

test('failure codes become plain sentences and never leak the raw message', () => {
  const errors = zh.dualSense.errors
  assert.equal(dualSenseErrorCode('DS5-HOST-002: the host has no DualSense tuning'), 'DS5-HOST-002')
  assert.equal(dualSenseErrorCode('timeout'), '')
  assert.equal(dualSenseErrorText('DS5-HOST-002: the host has no DualSense tuning', errors, 'loadFallback'), errors['DS5-HOST-002'])
  assert.equal(dualSenseErrorText('socket hang up', errors, 'loadFallback'), errors.loadFallback)
  assert.doesNotMatch(dualSenseErrorText('DS5-HOST-001: x', errors, 'loadFallback'), /[A-Za-z]{4,}.*DS5/)
  assert.equal(refusedSaveText(5, zh.dualSense.refused), zh.dualSense.refused['5'])
  assert.equal(refusedSaveText(99, zh.dualSense.refused), zh.dualSense.refused.other)
})

const flatten = (node, prefix = '') => Object.entries(node).flatMap(([name, value]) =>
  value && typeof value === 'object' && !Array.isArray(value)
    ? flatten(value, `${prefix}${name}.`)
    : [`${prefix}${name}`])

test('both languages define the same DualSense sentences', () => {
  assert.deepEqual(flatten(zh.dualSense).sort(), flatten(en.dualSense).sort())
})

test('dynamic sentence families are complete', () => {
  for (const locale of [zh, en]) {
    const status = locale.dualSense.status
    for (const state of Object.keys(status.states)) assert.ok(status.next[state], `next.${state}`)
    for (const code of ['DS5-HOST-001', 'DS5-HOST-002', 'DS5-HOST-003', 'DS5-HOST-004', 'DS5-HOST-005']) {
      assert.ok(locale.dualSense.errors[code], code)
    }
    for (const preset of ['default', 'competitive', 'immersive', 'legacy']) {
      assert.ok(locale.dualSense.tuning.presets[preset] && locale.dualSense.tuning.presets[`${preset}_hint`])
    }
    for (const id of [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]) assert.ok(locale.dualSense.tuning.groups[id], `group ${id}`)
    for (const card of ['haptics', 'triggers', 'motion', 'lighting', 'audio']) assert.ok(locale.dualSense.tuning.cards[card])
  }
})

test('the old component vocabulary is gone from the DualSense sentences', () => {
  const all = JSON.stringify([zh.dualSense, en.dualSense])
  assert.doesNotMatch(all, /Sidecar|HIDMaestro|VIIPER|修复组件|Repair component|本地组件包/i)
})
