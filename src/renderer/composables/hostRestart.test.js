import test from 'node:test'
import assert from 'node:assert/strict'
import { createHostRestart } from './hostRestart.js'

const live = (n) => ({ verdict: 'running', scm: { installed: true, belongsToInstall: true, state: 'running' }, process: { running: true }, api: { answers: true }, liveStreams: n })
function make({ answer = true, state = live(0), statusFails = false } = {}) {
  const calls = { asked: [], restart: [], userMode: [] }
  const host = createHostRestart({
    locale: 'zh',
    ask: async (options) => { calls.asked.push(options); if (!answer) throw new Error('cancel') },
    status: async () => { if (statusFails) throw new Error('down'); return state },
    restart: async (ack) => { calls.restart.push(ack) },
    userMode: async (ack) => { calls.userMode.push(ack) },
  })
  return { host, calls }
}

test('a restart asks first and passes the live stream acknowledgement', async () => {
  const { host, calls } = make({ state: live(2) })
  assert.equal(await host.run('restart'), true)
  assert.equal(calls.asked.length, 1)
  assert.match(calls.asked[0].message, /2 路串流/)
  assert.match(calls.asked[0].message, /管理员授权/)
  assert.equal(calls.asked[0].danger, true)
  assert.deepEqual(calls.restart, [true])
})

test('without live streams nothing is acknowledged', async () => {
  const { host, calls } = make()
  await host.run('restart')
  assert.deepEqual(calls.restart, [false])
  assert.doesNotMatch(calls.asked[0].message, /串流/)
})

test('cancel runs nothing', async () => {
  const { host, calls } = make({ answer: false, state: live(1) })
  assert.equal(await host.run('restart'), false)
  assert.equal(await host.run('user-mode'), false)
  assert.deepEqual(calls.restart, [])
  assert.deepEqual(calls.userMode, [])
})

test('the switch to user mode has its own confirmation and command', async () => {
  const { host, calls } = make({ state: live(1) })
  await host.run('user-mode')
  assert.match(calls.asked[0].title, /用户模式/)
  assert.match(calls.asked[0].message, /重启电脑后/)
  assert.deepEqual(calls.userMode, [true])
  assert.deepEqual(calls.restart, [])
})

test('an offline host still asks, with no stream count', async () => {
  const { host, calls } = make({ statusFails: true })
  await host.run('restart')
  assert.equal(calls.asked.length, 1)
  assert.deepEqual(calls.restart, [false])
})
