import assert from 'node:assert/strict'
import test from 'node:test'
import { isPrereleaseUnavailable, updateCheckFailureText } from './updateCheckError.js'

const texts = { prereleaseUnavailable: 'P', checkUpdatesFailed: 'G' }

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
