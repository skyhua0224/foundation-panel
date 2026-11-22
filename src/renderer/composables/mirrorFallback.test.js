import assert from 'node:assert/strict'
import test from 'node:test'
import { MIRROR_MISS_REASONS, mirrorFallbackText } from './mirrorFallback.js'
import { zh } from '../desktop/i18n/zh.js'
import { en } from '../desktop/i18n/en.js'

test('every reason has a sentence in the interface languages, and each names the source that took over', () => {
  for (const [name, texts] of [['zh', zh.updateDialog.mirrorFallback], ['en', en.updateDialog.mirrorFallback]]) {
    for (const reason of MIRROR_MISS_REASONS) {
      assert.match(mirrorFallbackText(reason, 'CNB', texts), /CNB/, `${name} ${reason}`)
      assert.doesNotMatch(mirrorFallbackText(reason, 'CNB', texts), /\{source\}/, `${name} ${reason}`)
    }
  }
})

test('the sentence for a version that Mirror酱 has not synced yet', () => {
  assert.equal(mirrorFallbackText('version_lag', 'CNB', zh.updateDialog.mirrorFallback), 'Mirror酱 暂未同步此版本，已改用 CNB 下载')
})

test('nothing is said without a known reason or before the source that took over is known', () => {
  assert.equal(mirrorFallbackText('', 'CNB', zh.updateDialog.mirrorFallback), '')
  assert.equal(mirrorFallbackText('bogus', 'CNB', zh.updateDialog.mirrorFallback), '')
  assert.equal(mirrorFallbackText('quota', '', zh.updateDialog.mirrorFallback), '')
  assert.equal(mirrorFallbackText('quota', 'CNB', undefined), '')
})
