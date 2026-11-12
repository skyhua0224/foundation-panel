import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import test from 'node:test'
import { bitrateChips, bitrateFromMbps, bitrateKbps, bitrateNudge, bitrateSliderMbps, bitrateText } from 'foundation-ui/settings'
import { en } from './i18n/en.js'
import { zh } from './i18n/zh.js'

// 串流 › 码率上限 (max_bitrate, Kbps; 0 = 自动, the client decides). Report 2026-11-08 「默认莫名其妙变成了65」「自动的时候这里显示有问题」:
// a host that never wrote the key (its /api/config has none) showed a made-up 50 Mbps, 0 was called 不限制 with a ∞ and the slider thumb at 1 Mbps, the bubble read Kbps ("50000").
const view = readFileSync(join(fileURLToPath(new URL('./', import.meta.url)), 'views/StreamView.vue'), 'utf8')

test('a config without the key shows 自动, not a made-up number: the page starts from the host default 0', () => {
  assert.match(view, /const configData = ref\(\{ hevc_mode: 2, av1_mode: 0, max_bitrate: '0',/)
  assert.doesNotMatch(view, /max_bitrate: '50000'/)
  // loadSettings keeps the default for a key the host did not send, so what shows is what the host does (client decides)
  assert.match(view, /if \(data\[key\] == null\) continue/)
  assert.equal(bitrateKbps(configData0()), 0); assert.equal(bitrateText(configData0(), zh.stream.bitrateAuto), '自动')
})
const configData0 = () => '0'

test('0 is 自动 everywhere on the page: the number, the chip and the slider; no 不限制, no ∞, no Kbps in the bubble', () => {
  assert.equal(zh.stream.bitrateAuto, '自动'); assert.equal(en.stream.bitrateAuto, 'Auto')
  assert.equal(zh.stream.bitrateUnlimited, undefined); assert.equal(en.stream.bitrateUnlimited, undefined)
  assert.doesNotMatch(view, /∞|bitrateUnlimited/)
  assert.match(view, /<MSlider :model-value="bitrateSliderProgress\(bitrateKbps\)" :min="0" :max="SLIDER_MAX_PROGRESS" :step="1" unit=""/)
  assert.match(view, /BITRATE_PRESET_MBPS = \[20, 50, 100, 0\]/); assert.match(view, /bitrateChips\(bitrateKbps\.value, t\.value\.stream\.bitrateAuto, BITRATE_PRESET_MBPS\)/)
  assert.deepEqual(bitrateChips(undefined, '自动', [20, 50, 100, 0]).map((c) => [c.value, c.label]), [[0, '自动'], [20000, '20 Mbps'], [50000, '50 Mbps'], [100000, '100 Mbps']])
})

test('a stored value is shown as it is and written as it is: custom values get a chip, the slider writes whole Kbps, strings from the host work', () => {
  assert.deepEqual(bitrateChips('65432', '自动', [20, 50, 100, 0]).map((c) => c.value), [0, 20000, 50000, 65432, 100000])
  assert.equal(bitrateText('65432', '自动'), '65.4 Mbps'); assert.equal(bitrateSliderMbps('65432'), 65)
  assert.equal(bitrateFromMbps(80), 80000); assert.match(view, /setCfg\('max_bitrate', String\(bitrateFromProgress\(v\)\)\)/)
  assert.match(view, /<MSeg :model-value="bitrateKbps" block/)
})

test('the step buttons keep the slider range: automatic steps up to 5 Mbps and 5 Mbps back to automatic, never to the top', () => {
  assert.equal(bitrateNudge(0, 5, 200), 5000); assert.equal(bitrateNudge(5000, -5, 200), 0); assert.equal(bitrateNudge(200000, 5, 200), 200000)
  assert.match(view, /String\(bitrateNudge\(bitrateKbps\.value, delta, 800\)\)/)
})
