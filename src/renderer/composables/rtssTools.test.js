import test from 'node:test'
import assert from 'node:assert/strict'
import { insertText, wrapTag, colorTag, sizeTag, metricGroups, sensorGroups, filterSensors, hwinfoOsdText, loadMonitoringConfig, saveMonitoringConfig, MONITORING_KEY, MONITORING_DEFAULTS } from './rtssTools.js'

test('the format buttons write the RTSS tags at the caret', () => {
  assert.deepEqual(wrapTag('abc', 1, 2, 'B'), { text: 'a<B>b<B0>c', caret: 5 })
  assert.deepEqual(wrapTag('', 0, 0, 'I'), { text: '<I><I0>', caret: 3 })
  assert.deepEqual(insertText('ab', 1, 1, '<C=FF0000>'), { text: 'a<C=FF0000>b', caret: 11 })
  assert.deepEqual(insertText('ab', 5, 9, '\\n'), { text: 'ab\\n', caret: 4 })
  assert.equal(colorTag('#ff8800'), '<C=FF8800>')
  assert.equal(sizeTag(24), '<S=24>')
  assert.equal(sizeTag(0), '')
})

test('the metrics come in the groups of the tool window, in the language of the panel', () => {
  const available = [{ id: 'stream_fps', group: 'session', label_zh: '串流帧率', label_en: 'Stream FPS' }, { id: 'cpu', group: 'process', label_zh: 'CPU', label_en: 'CPU' }, { id: 'x', group: 'other' }]
  assert.deepEqual(metricGroups(available, true).map((g) => [g.id, g.metrics.map((m) => m.label)]), [['session', ['串流帧率']], ['process', ['CPU']]])
  assert.equal(metricGroups(available, false)[0].label, 'Streaming')
  assert.deepEqual(metricGroups(null, true), [])
})

test('HWiNFO readings are grouped by sensor, found by search and written one per line', () => {
  const data = { sensors: [{ name_original: 'CPU' }, { name_user: 'GPU', name_original: 'x' }], readings: [{ sensor_index: 0, label_original: 'Core', unit: '°C', value: 50.04 }, { sensor_index: 1, label_original: 'Load', unit: '%', value: 7 }, { sensor_index: 1, label_user: 'Hot spot', unit: '°C', value: 61 }] }
  const groups = sensorGroups(data)
  assert.deepEqual(groups.map((g) => [g.name, g.readings.map((r) => r.index)]), [['CPU', [0]], ['GPU', [1, 2]]])
  assert.deepEqual(filterSensors(groups, 'hot').map((g) => g.readings.map((r) => r.label)), [['Hot spot']])
  assert.equal(filterSensors(groups, 'gpu')[0].readings.length, 2)
  assert.equal(hwinfoOsdText(data.readings.slice(0, 2)), 'Core: 50.0 °C\nLoad: 7.0 %')
  assert.deepEqual(sensorGroups(null), [])
})

test('the monitoring style is read from and written to the key of the tool window', () => {
  const store = new Map(), storage = { getItem: (k) => store.get(k) ?? null, setItem: (k, v) => store.set(k, v) }
  assert.deepEqual(loadMonitoringConfig(storage), { ...MONITORING_DEFAULTS, metrics: [...MONITORING_DEFAULTS.metrics] })
  saveMonitoringConfig({ ...MONITORING_DEFAULTS, header_text: 'Host' }, storage)
  assert.equal(JSON.parse(store.get(MONITORING_KEY)).header_text, 'Host')
  assert.equal(loadMonitoringConfig(storage).header_text, 'Host')
  store.set(MONITORING_KEY, '{broken')
  assert.equal(loadMonitoringConfig(storage).header_text, MONITORING_DEFAULTS.header_text)
})
