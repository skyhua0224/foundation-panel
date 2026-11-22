import assert from 'node:assert/strict'
import test from 'node:test'
import { createNrStatusPoll } from './nrStatusPoll.js'

for (const failed of [false, true]) {
  test(`a save invalidates a slow ${failed ? 'failed' : 'successful'} poll and reads the saved state`, async () => {
    const pending = Promise.withResolvers()
    let reads = 0
    const applied = []
    const { poll, refresh } = createNrStatusPoll(async current => {
      try {
        const value = ++reads === 1 ? await pending.promise : true
        if (current()) applied.push(value)
      } catch {
        if (current()) applied.push('offline')
      }
    })
    const finished = poll()
    void refresh()
    void refresh()
    assert.equal(reads, 1)
    if (failed) pending.reject(new Error('offline'))
    else pending.resolve(false)
    await finished
    assert.equal(reads, 2)
    assert.deepEqual(applied, [true])
    await poll()
    assert.deepEqual(applied, [true, true])
  })
}
