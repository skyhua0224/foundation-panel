import { test } from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

// Every slider of the Panel is the product slider (foundation-ui MSlider / MSliderItem: size m, inside icons at both ends where the setting has a less / more, a visible track, a value bubble).
// The shared component draws that by default; this keeps a page from going back to the thin track or from drawing a bare browser range input in a Panel window that has the design system.
const root = path.join(path.dirname(fileURLToPath(import.meta.url)), '..')
const walk = (dir) => fs.readdirSync(dir, { withFileTypes: true }).flatMap((e) => (e.isDirectory() ? walk(path.join(dir, e.name)) : [path.join(dir, e.name)]))
const files = walk(root).filter((f) => f.endsWith('.vue'))
const tags = (src) => [...src.matchAll(/<MSlider(?:Item)?\b[^>]*?(?:\/>|>)/gs)].map((m) => m[0])

test('no Panel slider asks for the thin xs or s track', () => {
  const bad = []
  for (const f of files) for (const t of tags(fs.readFileSync(f, 'utf8'))) if (/\bsize="(xs|s)"/.test(t)) bad.push(path.relative(root, f))
  assert.deepEqual(bad, [])
})

test('the bitrate, frame rate, scale and strength sliders carry their less / more icons', () => {
  const need = {
    'components/m3e/LiveSessionPage.vue': 'signal_cellular_alt_2_bar',
    'tool-window/tools/BitrateTool.vue': 'signal_cellular_alt_2_bar',
    'desktop/views/StreamView.vue': 'signal_cellular_alt_2_bar',
    'components/WebStreamSettings.vue': 'signal_cellular_alt_2_bar',
    'components/m3e/RtssPage.vue': 'slow_motion_video',
    'tool-window/tools/rtss/RtssFpsSection.vue': 'slow_motion_video',
    'components/m3e/DpiScaleSection.vue': 'text_decrease',
    'components/m3e/StreamTimerPage.vue': 'photo_size_select_small',
    'components/NrSessionControls.vue': 'photo_size_select_small',
  }
  for (const [rel, icon] of Object.entries(need)) {
    const src = fs.readFileSync(path.join(root, rel), 'utf8')
    assert.match(src, new RegExp(`icon-start="${icon}" icon-end="`), rel)
  }
})

test('every bitrate slider of the Panel runs on the shared segmented scale with the marks under its chips', () => {
  for (const rel of ['components/m3e/LiveSessionPage.vue', 'components/WebStreamSettings.vue', 'tool-window/tools/BitrateTool.vue']) {
    const src = fs.readFileSync(path.join(root, rel), 'utf8')
    assert.match(src, /bitrateSliderSpan\(/, rel); assert.match(src, /bitrateChipMarks\(/, rel); assert.match(src, /bitrateFromProgress\(position\)/, rel); assert.match(src, /:marks="bitrateMarks"/, rel)
    assert.doesNotMatch(src, /<MSliderItem[^>]*:max="(BITRATE\.max|150000|BITRATE_LIMITS\.MAX)"/, `${rel}: a linear Kbps track`)
  }
  assert.match(fs.readFileSync(path.join(root, 'desktop/views/StreamView.vue'), 'utf8'), /BITRATE_PRESET_MBPS = \[20, 50, 100, 0\][\s\S]*bitrateChips\([^)]*BITRATE_PRESET_MBPS\)/)
})

test('the sliders write when they did before: the commit and the model stay on the same tags', () => {
  const live = fs.readFileSync(path.join(root, 'components/m3e/LiveSessionPage.vue'), 'utf8')
  assert.match(live, /<MSliderItem v-model="bitrateProgress"[^>]*:readout-text="formatBitrate\(bitrate\)"/)
  const stream = fs.readFileSync(path.join(root, 'desktop/views/StreamView.vue'), 'utf8')
  assert.match(stream, /:marks="bitrateChipMarks\(BITRATE_PRESET_MBPS\)" :off-text="t\.stream\.bitrateAuto" size="m"/)
})
