import assert from 'node:assert/strict'
import test from 'node:test'
import { nrDegradedReason, nrOverlayText, nrStatusLabel } from './nrOverlayMessages.js'

const zh = nrOverlayText('zh')
const en = nrOverlayText('en')

test('a saved switch explains when NR can start without claiming the stream is already enhanced', () => {
  for (const text of [zh, en]) {
    assert.equal(nrStatusLabel(text, 'idle', true), text.enabledIdle)
    assert.equal(nrStatusLabel(text, 'no_pipeline', true), text.enabledNoPipeline)
    assert.equal(nrStatusLabel(text, 'no_pipeline', false), text.states.no_pipeline)
    for (const state of ['warming_up', 'active', 'stopping', 'blocked', 'app_disabled', 'degraded', 'unavailable']) {
      assert.equal(nrStatusLabel(text, state, true), text.states[state])
    }
  }
})

test('component-unavailable degradation explains the permission fix', () => {
  const pipeline = { nr_reason: 'component_unavailable' }
  assert.deepEqual(nrDegradedReason(pipeline, zh), { main: zh.componentUnavailable, detail: '' })
  assert.deepEqual(nrDegradedReason(pipeline, en), { main: en.componentUnavailable, detail: '' })
})

test('other degradation reasons keep the fallback and show their code', () => {
  assert.deepEqual(nrDegradedReason({ nr_reason: 'runtime_untrusted' }, en), {
    main: en.fallback,
    detail: 'runtime_untrusted'
  })
  assert.deepEqual(nrDegradedReason({}, en), { main: en.fallback, detail: '' })
  assert.deepEqual(nrDegradedReason(null, zh), { main: zh.fallback, detail: '' })
})
