import test from 'node:test'
import assert from 'node:assert/strict'
import { exportPanelLogs } from './panelLogExport.js'

const texts = {
  exportSuccess: 'Logs exported successfully',
  exportComplete: 'Export Complete',
  exportFailed: 'Failed to export logs',
  dialogError: 'Error',
}

test('cancelling the log export save dialog shows nothing', async () => {
  const alerts = []
  const exported = await exportPanelLogs({ invoke: async () => { throw 'cancelled' }, format: 'txt', texts, alert: async (...args) => { alerts.push(args) } })

  assert.equal(exported, false)
  assert.deepEqual(alerts, [])
})

test('a written log export names the file', async () => {
  const alerts = []
  const exported = await exportPanelLogs({ invoke: async () => 'Logs exported to: panel.txt', format: 'json', texts, alert: async (...args) => { alerts.push(args) } })

  assert.equal(exported, true)
  assert.deepEqual(alerts, [['Logs exported to: panel.txt', 'Export Complete', { type: 'info' }]])
})

test('a real log export failure is shown with its reason', async () => {
  const alerts = []
  const exported = await exportPanelLogs({ invoke: async () => { throw 'write failed: disk full' }, format: 'txt', texts, alert: async (...args) => { alerts.push(args) } })

  assert.equal(exported, false)
  assert.deepEqual(alerts, [['Failed to export logs: write failed: disk full', 'Error', { type: 'error' }]])
})
