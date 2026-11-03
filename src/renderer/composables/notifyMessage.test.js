import test from 'node:test'
import assert from 'node:assert/strict'
import { ElMessageBox } from './notifyMessage.js'
import { useDialogs } from 'foundation-ui/dialog'

const next = async () => { for (let i = 0; i < 20 && !useDialogs().state.queue.length; i += 1) await new Promise((r) => setTimeout(r, 5)); return useDialogs().state.queue[0] }

test('the old message box call asks the dialog of the window with its own options and resolves confirm', async () => {
  const answer = ElMessageBox.confirm('会移除驱动', '卸载？', { type: 'warning', confirmButtonText: '卸载' })
  const dialog = await next()
  assert.equal(dialog.title, '卸载？')
  assert.equal(dialog.confirmButtonText, '卸载')
  assert.equal(dialog.cancel, true)
  useDialogs().settle(dialog.id, 'confirm')
  assert.equal(await answer, 'confirm')
})

test('cancel rejects cancel, an alert has no cancel button, and a caller that tells close apart gets close', async () => {
  const a = ElMessageBox.confirm('x', 'y')
  useDialogs().settle((await next()).id, 'cancel')
  await assert.rejects(a, (e) => e === 'cancel')
  const b = ElMessageBox.alert('完成', '重启')
  const alert = await next()
  assert.equal(alert.cancel, false)
  useDialogs().settle(alert.id, 'close')
  assert.equal(await b, 'confirm')
  const c = ElMessageBox({ title: 't', message: 'm', showCancelButton: true, distinguishCancelAndClose: true, closeOnClickModal: false })
  const choice = await next()
  assert.equal(choice.persistent, true)
  useDialogs().settle(choice.id, 'close')
  await assert.rejects(c, (e) => e === 'close')
})
