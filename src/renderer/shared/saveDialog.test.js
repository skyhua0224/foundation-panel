import test from 'node:test'
import assert from 'node:assert/strict'
import { saveThroughDialog } from './saveDialog.js'

test('a save dialog answer is passed through', async () => {
  assert.equal(await saveThroughDialog(async (command, args) => `${command}:${args.format}`, 'export_logs', { format: 'txt' }), 'export_logs:txt')
})

test('closing the save dialog is a quiet cancel, not an error', async () => {
  assert.equal(await saveThroughDialog(async () => { throw 'cancelled' }, 'export_logs', { format: 'txt' }), null)

  const aborted = Object.assign(new Error('The user aborted a request.'), { name: 'AbortError' })
  assert.equal(await saveThroughDialog(async () => { throw aborted }, 'save_text_file', {}), null)
})

test('a real save failure keeps its reason', async () => {
  await assert.rejects(
    saveThroughDialog(async () => { throw 'write failed: disk full' }, 'save_text_file', {}),
    (error) => error === 'write failed: disk full',
  )
})
