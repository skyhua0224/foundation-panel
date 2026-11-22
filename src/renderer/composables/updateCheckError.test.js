import assert from 'node:assert/strict'
import test from 'node:test'
import { hasUpdate, isCurrentVersionUnknown, isPrereleaseUnavailable, updateCheckFailureText } from './updateCheckError.js'

const texts = { prereleaseUnavailable: 'P', currentVersionUnknown: 'C', checkUpdatesFailed: 'G' }

test('the native prerelease_unavailable error is told apart from other failures', () => {
  assert.equal(isPrereleaseUnavailable('prerelease_unavailable: HTTP 状态码 403'), true)
  assert.equal(isPrereleaseUnavailable(new Error('prerelease_unavailable: 请求超时')), true)
  assert.equal(isPrereleaseUnavailable('prerelease_unavailable'), true)
  assert.equal(isPrereleaseUnavailable('prerelease_unavailable_extra'), false)
  assert.equal(isPrereleaseUnavailable('请求失败: timeout'), false)
  assert.equal(isPrereleaseUnavailable(null), false)
  assert.equal(isPrereleaseUnavailable(undefined), false)
})

test('the failure text names the prerelease information only when that is the reason', () => {
  assert.equal(updateCheckFailureText('prerelease_unavailable: HTTP 状态码 403', texts), 'P')
  assert.equal(updateCheckFailureText('HTTP 状态码 500', texts), 'G')
})

test('a check result that says "up to date" is never an update to show on the overview or the host tile', () => {
  assert.equal(hasUpdate(null), false)
  assert.equal(hasUpdate(undefined), false)
  assert.equal(hasUpdate({ version: 'v2026.1116.43312', is_latest: true }), false)
  assert.equal(hasUpdate({ version: 'v2026.1116.43312', is_latest: false }), true)
  assert.equal(hasUpdate({ version: 'v2026.1116.43312' }), true)
})

test('an unreadable installed version has its own sentence', () => {
  assert.equal(isCurrentVersionUnknown('current_version_unknown: host did not answer'), true)
  assert.equal(isCurrentVersionUnknown(new Error('current_version_unknown')), true)
  assert.equal(isCurrentVersionUnknown('current_version_unknown_extra'), false)
  assert.equal(updateCheckFailureText('current_version_unknown: x', texts), 'C')
})
