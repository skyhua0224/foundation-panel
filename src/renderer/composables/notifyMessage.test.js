import test from 'node:test'
import assert from 'node:assert/strict'
import { ElMessage, ElMessageBox, ElNotification, showBusy } from './notifyMessage.js'
import { useDialogs } from 'foundation-ui/dialog'
import { useNotify } from 'foundation-ui/notify'

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

test('a destructive call carries danger and its verb to the dialog, so the host draws the error-colour button', async () => {
  const answer = ElMessageBox.confirm('会从这台电脑上移除驱动', '卸载？', { type: 'warning', danger: true, confirmButtonText: '卸载' })
  const dialog = await next()
  assert.equal(dialog.danger, true)
  assert.equal(dialog.confirmButtonText, '卸载')
  useDialogs().settle(dialog.id, 'cancel')
  await assert.rejects(answer, (e) => e === 'cancel')
})

const settled = () => new Promise((resolve) => setTimeout(resolve, 20))
const snacks = () => useNotify().state.queue

test('a message with duration 0 stays until its handle is closed', async () => {
  const handle = ElMessage({ message: '清理中', duration: 0 })
  await settled()
  assert.equal(snacks().length, 1)
  assert.equal(snacks()[0].message, '清理中')
  assert.equal(snacks()[0].timeout, 0)
  await handle.close()
  assert.equal(snacks().length, 0)
  ElMessage.error('失败')
  await settled()
  assert.equal(snacks()[0].timeout, 8000)
  useNotify().dismissSnack(snacks()[0].id)
})

test('a notification with a click and an action text becomes a snackbar whose action runs the click', async () => {
  let clicked = 0
  ElNotification({ title: '新版本', message: '当前 1.0', duration: 0, actionText: '更新', onClick: () => { clicked += 1 } })
  await settled()
  const snack = snacks()[0]
  assert.equal(snack.message, '新版本 · 当前 1.0')
  assert.equal(snack.action, '更新')
  assert.equal(snack.timeout, 0)
  useNotify().runSnackAction(snack.id)
  assert.equal(clicked, 1)
  assert.equal(snacks().length, 0)
})

test('a wait with words rewrites one snackbar in place and leaves with close', async () => {
  const busy = showBusy('准备中')
  await settled()
  assert.equal(snacks().length, 1)
  assert.equal(snacks()[0].timeout, 0)
  const id = snacks()[0].id
  busy.setText('下载 50%')
  await settled()
  assert.equal(snacks().length, 1)
  assert.equal(snacks()[0].id, id)
  assert.equal(snacks()[0].message, '下载 50%')
  await busy.close()
  assert.equal(snacks().length, 0)
  await busy.setText('迟到的进度')
  assert.equal(snacks().length, 0)
})
