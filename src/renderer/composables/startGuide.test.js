import './_testing/foundationUi.mjs'
import test from 'node:test'
import assert from 'node:assert/strict'

const { armStartGuide, consumeStartGuide, deliverPendingStartGuide, guideOnLaunch } = await import('./startGuide.js')
const { buildGuideSteps, clearProgress, readProgress, startIndex, writeProgress } = await import('foundation-ui/vue/guide.js')

const memory = () => {
  const stored = new Map()
  return {
    getItem: (key) => (stored.has(key) ? stored.get(key) : null),
    setItem: (key, value) => stored.set(key, String(value)),
    removeItem: (key) => stored.delete(key),
  }
}

test('--start-guide is the fresh flow at its first step, whatever was saved', () => {
  const storage = memory()
  const key = 'foundation.guide'
  writeProgress(storage, key, { step: 'pair', finished: true, visited: ['intro', 'setup', 'pair'], version: '9.0.0', mode: 'upgrade', device: 'windows' })
  assert.equal(consumeStartGuide(), false)
  armStartGuide()
  armStartGuide()
  assert.equal(consumeStartGuide(), true)
  assert.equal(consumeStartGuide(), false)

  const entry = guideOnLaunch({ fromStart: true, entry: { mode: 'upgrade', skip: ['setup', 'install'] } })
  assert.deepEqual(entry, { mode: 'fresh', skip: [], fromStart: true })
  clearProgress(storage, key)
  const steps = buildGuideSteps({ mode: entry.mode, skip: [...entry.skip, 'account'] })
  assert.equal(steps[0].id, 'intro')
  assert.equal(steps.some((step) => step.id === 'whatsnew'), false)
  assert.equal(startIndex(steps, readProgress(storage, key)), 0)

  const saved = readProgress(storage, key)
  writeProgress(storage, key, { step: 'remote', finished: false, visited: ['intro', 'remote'], version: '9.0.0', mode: 'fresh' })
  const kept = guideOnLaunch({ fromStart: false, entry: { mode: 'upgrade', skip: ['account'] } })
  assert.deepEqual(kept, { mode: 'upgrade', skip: ['account'] })
  // a position of the old flow (the remote access step) continues at the first step of the new flow still to do: the client install
  assert.equal(startIndex(buildGuideSteps({ mode: 'fresh' }), readProgress(storage, key)), buildGuideSteps({ mode: 'fresh' }).findIndex((step) => step.id === 'install'))
  assert.equal(saved.step, null)
})

test('the host slot opens the guide once, and a failed call does not', async () => {
  let slot = true
  const opened = []
  const invoke = async (cmd) => {
    assert.equal(cmd, 'take_pending_start_guide')
    const value = slot
    slot = false
    return value
  }
  assert.equal(await deliverPendingStartGuide(invoke, () => opened.push('a')), true)
  assert.equal(await deliverPendingStartGuide(invoke, () => opened.push('b')), false)
  assert.deepEqual(opened, ['a'])
  assert.equal(await deliverPendingStartGuide(async () => { throw new Error('ipc') }, () => opened.push('c')), false)
  assert.deepEqual(opened, ['a'])
})
