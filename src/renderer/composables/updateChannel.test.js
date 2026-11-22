import './_testing/foundationUi.mjs'
import test from 'node:test'
import assert from 'node:assert/strict'

const { CHANNEL_CHOSEN_KEY, CHANNEL_EVENT, INCLUDE_PRERELEASE_KEY, channelChosen, rememberChannel, saveUpdateChannel } = await import('./updateChannel.js')

const memory = () => { const m = new Map(); return { getItem: (k) => (m.has(k) ? m.get(k) : null), setItem: (k, v) => m.set(k, String(v)), m } }

test('a person who chose the channel before is told from one who did not: a mark, or a stored 测试版', () => {
  assert.equal(channelChosen(memory()), false)
  const stable = memory(); stable.setItem(INCLUDE_PRERELEASE_KEY, 'false')
  assert.equal(channelChosen(stable), false, 'the stored default is not a choice')
  const beta = memory(); beta.setItem(INCLUDE_PRERELEASE_KEY, 'true')
  assert.equal(channelChosen(beta), true, 'a stored 测试版 can only have been picked')
  const marked = memory(); marked.setItem(CHANNEL_CHOSEN_KEY, '1')
  assert.equal(channelChosen(marked), true)
})

test('saving the channel writes the native preference and the stored copy, leaves the mark and tells the window', async () => {
  const calls = []; const storage = memory(); const events = []
  await saveUpdateChannel(async (name, args) => { calls.push([name, args]) }, true, { storage, win: { dispatchEvent: (e) => events.push([e.type, e.detail]) } })
  assert.deepEqual(calls, [['set_include_prerelease_preference', { include: true }]])
  assert.equal(storage.getItem(INCLUDE_PRERELEASE_KEY), 'true'); assert.equal(storage.getItem(CHANNEL_CHOSEN_KEY), '1')
  assert.deepEqual(events, [[CHANNEL_EVENT, true]])
  await saveUpdateChannel(async () => {}, false, { storage, win: null })
  assert.equal(storage.getItem(INCLUDE_PRERELEASE_KEY), 'false'); assert.equal(channelChosen(storage), true, 'a choice of 稳定版 leaves its mark')
  // the native side refusing: nothing is stored
  const fresh = memory()
  await assert.rejects(saveUpdateChannel(async () => { throw new Error('no') }, true, { storage: fresh, win: null }))
  assert.equal(fresh.getItem(CHANNEL_CHOSEN_KEY), null)
})

test('a storage that refuses one write keeps neither the other write nor the native save from happening', async () => {
  const refusing = (failKey) => { const m = memory(); return { ...m, setItem: (k, v) => { if (k === failKey) throw new Error('QuotaExceededError'); m.setItem(k, v) } } }
  const noMark = refusing(CHANNEL_CHOSEN_KEY)
  assert.doesNotThrow(() => rememberChannel(true, noMark))
  assert.equal(noMark.getItem(INCLUDE_PRERELEASE_KEY), 'true', 'the preference is stored although the mark is refused')
  const noPref = refusing(INCLUDE_PRERELEASE_KEY)
  assert.doesNotThrow(() => rememberChannel(false, noPref))
  assert.equal(noPref.getItem(CHANNEL_CHOSEN_KEY), '1', 'the mark is stored although the preference is refused')
  const calls = []
  await saveUpdateChannel(async (name) => { calls.push(name) }, true, { storage: refusing(CHANNEL_CHOSEN_KEY), win: null })
  assert.deepEqual(calls, ['set_include_prerelease_preference'])
})

test('the sidebar saves the channel through the same helper, before the native call and outside it', async () => {
  const { readFile } = await import('node:fs/promises')
  const src = await readFile(new URL('./useSidebarState.js', import.meta.url), 'utf8')
  const body = src.slice(src.indexOf('const setIncludePrerelease'), src.indexOf('const initAdminStatus'))
  assert.ok(body.indexOf('rememberChannel(value)') > 0 && body.indexOf('rememberChannel(value)') < body.indexOf("invoke('set_include_prerelease_preference'"))
  assert.doesNotMatch(body, /localStorage\.setItem/, 'no direct storage write left in front of the native save')
})
