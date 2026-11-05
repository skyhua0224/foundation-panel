import test from 'node:test'
import assert from 'node:assert/strict'
import { applyVddEntries, leaves, setPath, vddEntries } from './useVddPending.js'

const saved = { monitors: { count: 1 }, gpu: { friendlyname: '' }, global: { g_refresh_rate: [60, 120] }, resolutions: { resolution: [{ width: 1920, height: 1080 }] }, colour: { HDRPlus: false, ColourFormat: 'RGB' }, edid: { Vrr: false } }

test('a list of the settings is one value and every changed value is one entry with its words', () => {
  const now = JSON.parse(JSON.stringify(saved))
  now.resolutions.resolution.push({ width: 2560, height: 1440 })
  now.colour.HDRPlus = true
  now.global.g_refresh_rate = [60, 120, 144]
  const entries = vddEntries(saved, now, 'zh')
  assert.deepEqual(entries.map((e) => e.key), ['global.g_refresh_rate', 'resolutions.resolution', 'colour.HDRPlus'])
  assert.equal(entries[0].label, '刷新率列表')
  assert.equal(entries[1].afterText, '1920×1080, 2560×1440')
  assert.equal(vddEntries(saved, saved, 'zh').length, 0)
})

test('the entries apply to the settings the host holds and the old values write back', () => {
  const now = JSON.parse(JSON.stringify(saved))
  now.edid.Vrr = true
  now.gpu.friendlyname = 'NVIDIA RTX'
  const entries = vddEntries(saved, now, 'en')
  assert.deepEqual(applyVddEntries(saved, entries, 'after'), now)
  assert.deepEqual(applyVddEntries(now, entries, 'before'), saved)
})

test('leaves and setPath walk the nested settings', () => {
  assert.equal(leaves(saved)['colour.ColourFormat'], 'RGB')
  const o = {}
  setPath(o, 'a.b.c', [1])
  assert.deepEqual(o, { a: { b: { c: [1] } } })
})
