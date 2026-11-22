import test from 'node:test'
import assert from 'node:assert/strict'
import { afterCheck, afterOpened, noticeView } from 'foundation-ui/update'
import { helpTileTexts, noticeAfterTile, updateTile, aboutLine } from 'foundation-ui/help-tiles'
import { helpUpdateState, helpUpdateView } from './helpUpdate.js'

const T = helpTileTexts('zh')

test('the tile state follows the check: checking first, then a waiting version, a failure, an answer, nothing', () => {
  assert.equal(helpUpdateState({}), 'idle')
  assert.equal(helpUpdateState({ answered: true }), 'latest')
  assert.equal(helpUpdateState({ answered: true, waiting: true }), 'available')
  assert.equal(helpUpdateState({ failed: true }), 'error')
  assert.equal(helpUpdateState({ failed: true, answered: true }), 'error')
  assert.equal(helpUpdateState({ waiting: true, checking: true }), 'checking')
})

test('every state of the tile reads in the words of the page, with the action on the tile', () => {
  const view = (flags) => updateTile(helpUpdateView(flags, { installed: 'v2026.1122.101010', next: '2026.1123.090909', checkedAt: 0 }), T)
  assert.deepEqual(['idle', 'latest', 'available', 'error', 'checking'].map((state) => {
    const flags = { idle: {}, latest: { answered: true }, available: { waiting: true }, error: { failed: true }, checking: { checking: true } }[state]
    const t = view(flags)
    return [t.state, t.value, t.sub, t.action.label]
  }), [
    ['idle', '2026.1122', '尚未检查', '检查更新'],
    ['latest', '2026.1122', '已是最新', '检查更新'],
    ['available', '2026.1123', '有新版本', '更新'],
    ['error', '2026.1122', '检查失败', '重试'],
    ['checking', '2026.1122', '正在检查…', ''],
  ])
})

test('pressing 更新 on the tile counts as opening the update: the dot goes out, the update still waits, a newer version brings it back', () => {
  const installed = '2026.1122.101010'
  let state = afterCheck({ available: '', seen: '' }, { version: '2026.1123.090909', is_latest: false }, { installed })
  assert.equal(noticeView(state, { installed }).dot, true)
  const tile = updateTile(helpUpdateView({ waiting: noticeView(state, { installed }).waiting }, { installed, next: noticeView(state, { installed }).version }), T)
  state = noticeAfterTile(state, tile)
  assert.deepEqual(noticeView(state, { installed }), { waiting: true, version: '2026.1123.090909', dot: false })
  assert.deepEqual(state, afterOpened(afterCheck({ available: '', seen: '' }, { version: '2026.1123.090909' }, { installed })))
  state = afterCheck(state, { version: '2026.1124.080808', is_latest: false }, { installed })
  assert.equal(noticeView(state, { installed }).dot, true)
})

test('a tile that only shows the state leaves the dot as it is', () => {
  const installed = '1.0.0'
  const state = afterCheck({ available: '', seen: '' }, { version: '1.1.0' }, { installed })
  for (const flags of [{}, { answered: true }, { failed: true }, { checking: true }]) assert.equal(noticeAfterTile(state, updateTile(helpUpdateView(flags, { installed }), T)), state)
})

test('the second line of 关于 carries the update state only while one waits', () => {
  assert.equal(aboutLine('v2026.1122.101010', { state: 'available' }, T), '2026.1122 · 有新版本')
  assert.equal(aboutLine('v2026.1122.101010', { state: 'latest' }, T), '2026.1122')
})
