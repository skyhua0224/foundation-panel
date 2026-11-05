import test from 'node:test'
import assert from 'node:assert/strict'
import { combineDiagnosticsRecords } from './diagnosticsRecords.js'

test('diagnostics combine the host text log and the panel entries as two sources', () => {
  const records = combineDiagnosticsRecords(
    '[2026-11-02 16:45:58.696]: Info: Windows system message (zh-CN): 系统找不到指定的文件。',
    [
      {
        timestamp: '2026-11-02 16:58:10.100',
        level: 'error',
        message: '面板读取 VDD 状态失败: 系统找不到指定的文件。',
        target: 'sunshine_gui::vdd',
        file: 'vdd.rs',
        line: 116,
      },
    ],
  )

  assert.equal(records.length, 2)
  assert.deepEqual(records.map((record) => record.source.id), ['host-text', 'panel'])
  assert.deepEqual(records.map((record) => record.domain), ['platform', 'ux'])
  assert.deepEqual(records.map((record) => record.severity), [9, 17])
  assert.equal(records[1].attributes['panel.file'], 'vdd.rs')
  assert.match(records[1].body, /系统找不到指定的文件/)
})
