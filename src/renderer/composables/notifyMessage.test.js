import test from 'node:test'
import assert from 'node:assert/strict'
import { ElMessage, ElMessageBox, ElNotification } from './notifyMessage.js'
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

test('a notification that waits for the person stands in the status notices and never holds the snackbar queue', async () => {
  let clicked = 0
  const handle = ElNotification({ title: '新版本', message: '当前 1.0，最新 2.0', duration: 0, actionText: '更新', standing: 'tool-update', revision: '2.0', onClick: () => { clicked += 1 } })
  await settled()
  assert.equal(snacks().length, 0)
  const systems = useNotify().state.systems
  assert.equal(systems['tool-update'].title, '新版本')
  assert.equal(systems['tool-update'].text, '当前 1.0，最新 2.0')
  assert.equal(systems['tool-update'].dismissible, true)
  assert.equal(systems['tool-update'].revision, '2.0')
  ElMessage.error('另一件事失败了')
  await settled()
  assert.equal(snacks()[0].message, '另一件事失败了')
  useNotify().dismissSnack(snacks()[0].id)
  systems['tool-update'].actions[0].onClick(systems['tool-update'])
  assert.equal(clicked, 1)
  await handle.close()
  assert.equal(systems['tool-update'], undefined)
})

test('a dismissed standing notice stays away until what it reports changes', async () => {
  const offer = (revision) => ElNotification({ title: '新版本', message: `最新 ${revision}`, duration: 0, actionText: '更新', standing: 'tool-update-2', revision, onClick: () => {} })
  await offer('2.0')
  await settled()
  useNotify().dismissSystem('tool-update-2')
  assert.equal(useNotify().state.systems['tool-update-2'], undefined)
  await offer('2.0')
  await settled()
  assert.equal(useNotify().state.systems['tool-update-2'], undefined)
  await offer('3.0')
  await settled()
  assert.equal(useNotify().state.systems['tool-update-2'].revision, '3.0')
  useNotify().clearSystem('tool-update-2')
})

test('a message with an id is replaced where it stands by the next call with that id, and its handle closes it', async () => {
  const progress = ElMessage({ message: '正在检查更新...', id: 'update-check', duration: 0 })
  await settled()
  assert.equal(snacks().length, 1)
  assert.equal(snacks()[0].id, 'update-check')
  assert.equal(snacks()[0].timeout, 0)
  ElMessage.error({ message: '检查更新失败', id: 'update-check' })
  await settled()
  assert.equal(snacks().length, 1, 'one message, not a second one waiting behind the first')
  assert.equal(snacks()[0].id, 'update-check')
  assert.equal(snacks()[0].message, '检查更新失败')
  assert.equal(snacks()[0].timeout, 8000)
  await progress.close()
  assert.equal(snacks().length, 0)
  // without an id every call is a message of its own
  ElMessage.info('甲'); ElMessage.info('乙')
  await settled()
  assert.equal(snacks().length, 2)
  for (const s of [...snacks()]) useNotify().dismissSnack(s.id)
})

// The snackbar of the window must not start a second entrance when its enter class goes (the base .m-snackbar rule carries m-dialog-in): foundation-ui keeps the animation on the enter class only.
test('the installed foundation-ui snackbar has no animation of its own, so a check for updates shows one entrance only', async () => {
  const { readFileSync } = await import('node:fs')
  const { fileURLToPath } = await import('node:url')
  const css = readFileSync(fileURLToPath(new URL('../css/notify.css', import.meta.resolve('foundation-ui/notify'))), 'utf8')
  assert.match(css, /\.m-snackbar\[popover\] \{ animation: none; \}/)
  assert.match(css, /\.m-snackbar\[popover\]\.m-snack-enter-active \{ animation: m-snack-in /)
})
