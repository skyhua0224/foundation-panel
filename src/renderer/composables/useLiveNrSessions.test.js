import assert from 'node:assert/strict'
import test from 'node:test'
import { ref } from 'vue'
import { useLiveNrSessions } from './useLiveNrSessions.js'

for (const stale of ['success', 'failure']) {
  test(`a late poll ${stale} cannot undo the refresh after a global NR save`, async () => {
    const reads = [], selections = []
    const { nr, pollNr } = useLiveNrSessions(ref(true), (command, args) => {
      if (command === 'nr_overlay_settings') return Promise.resolve({ target: null })
      if (command === 'nr_overlay_select_session') { selections.push(args.id); return Promise.resolve() }
      assert.equal(command, 'nr_live_status')
      const pending = Promise.withResolvers()
      reads.push(pending)
      return pending.promise
    })
    const oldPoll = pollNr(), afterSave = pollNr()
    const session = { id: 7, hdr_mode: 'sdr' }
    reads[1].resolve({ nr_enabled: true, has_streams: true, pipelines: [session] })
    await afterSave
    if (stale === 'success') reads[0].resolve({ nr_enabled: false, has_streams: false, pipelines: [] })
    else reads[0].reject(new Error('old connection timed out'))
    await oldPoll
    assert.deepEqual({ ...nr }, { enabled: true, hasStreams: true, list: [session], online: true, selected: 7 })
    assert.deepEqual(selections, [7])
    const next = pollNr()
    reads[2].resolve({ nr_enabled: false, has_streams: false, pipelines: [] })
    await next
    assert.equal(nr.enabled, false)
    assert.equal(nr.selected, null)
  })
}

const stream = id => ({ id, hdr_mode: 'sdr' })
function delayedSelection(initialIds) {
  const started = Promise.withResolvers(), ack = Promise.withResolvers(), writes = []
  let ids = initialIds
  const sessions = useLiveNrSessions(ref(true), async (command, args) => {
    if (command === 'nr_overlay_settings') return { target: null }
    if (command === 'nr_live_status') return { nr_enabled: true, has_streams: true, pipelines: ids.map(stream) }
    assert.equal(command, 'nr_overlay_select_session')
    writes.push(args.id)
    if (writes.length === 1) { started.resolve(); await ack.promise }
  })
  return { ...sessions, writes, started: started.promise, acknowledge: ack.resolve, setIds(next) { ids = next } }
}

test('a second stream joining during automatic selection keeps the acknowledged target', async () => {
  const s = delayedSelection([7])
  const selecting = s.pollNr()
  await s.started
  s.setIds([7, 8])
  await s.pollNr()
  s.acknowledge()
  await selecting
  assert.equal(s.nr.selected, 7)
  assert.deepEqual(s.writes, [7])
})

test('a selection acknowledgement cannot restore a departed stream', async () => {
  const s = delayedSelection([7, 8])
  await s.pollNr()
  const selecting = s.selectNr(7)
  await s.started
  s.setIds([8, 9])
  await s.pollNr()
  s.acknowledge()
  await selecting
  assert.equal(s.nr.selected, null)
})

test('rapid target changes coalesce and leave the native overlay and page on the final choice', async () => {
  const s = delayedSelection([7, 8, 9])
  await s.pollNr()
  const first = s.selectNr(7)
  await s.started
  const second = s.selectNr(8), last = s.selectNr(9)
  await Promise.resolve()
  assert.deepEqual(s.writes, [7])
  s.acknowledge()
  await Promise.all([first, second, last])
  assert.deepEqual(s.writes, [7, 9])
  assert.equal(s.nr.selected, 9)
})
