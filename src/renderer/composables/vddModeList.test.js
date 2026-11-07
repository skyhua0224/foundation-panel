import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { addEntry, cleanList, editEntry, fromResolutionNodes, moveEntry, normalizeRate, normalizeResolution, parseResolution, removeEntry, sortList, toResolutionNodes, MAX_ENTRIES } from './vddModeList.js'

test('a resolution is read in the usual spellings and written as widthxheight', () => {
  assert.deepEqual(parseResolution('1920x1080'), { width: 1920, height: 1080 })
  assert.deepEqual(parseResolution(' 2560 × 1440 '), { width: 2560, height: 1440 })
  assert.equal(parseResolution('1920'), null)
  assert.equal(parseResolution('axb'), null)
  assert.deepEqual(normalizeResolution('1920X1080'), { value: '1920x1080' })
  assert.deepEqual(normalizeResolution('01920x1080'), { value: '1920x1080' })
})

test('resolution ranges: below 320 or above 16384 on either side is refused', () => {
  assert.equal(normalizeResolution('319x1080').error, 'range')
  assert.equal(normalizeResolution('1920x16385').error, 'range')
  assert.equal(normalizeResolution('0x0').error, 'range')
  assert.equal(normalizeResolution('320x320').value, '320x320')
  assert.equal(normalizeResolution('16384x16384').value, '16384x16384')
  assert.equal(normalizeResolution('abc').error, 'invalid')
})

test('refresh rates: 1 to 480, up to three decimals, comma and Hz accepted, canonical text', () => {
  assert.deepEqual(normalizeRate('60'), { value: '60' })
  assert.deepEqual(normalizeRate('59.94'), { value: '59.94' })
  assert.deepEqual(normalizeRate('119,88'), { value: '119.88' })
  assert.deepEqual(normalizeRate('144 Hz'), { value: '144' })
  assert.deepEqual(normalizeRate('60.0'), { value: '60' })
  assert.equal(normalizeRate('0').error, 'range')
  assert.equal(normalizeRate('481').error, 'range')
  assert.equal(normalizeRate('59.9444').error, 'invalid')
  assert.equal(normalizeRate('-60').error, 'invalid')
  assert.equal(normalizeRate('').error, 'invalid')
})

test('adding: the value goes to the end, a duplicate (also in another spelling) and a bad value change nothing', () => {
  const list = ['1920x1080', '1280x720']
  const added = addEntry('resolution', list, '2560 × 1440')
  assert.deepEqual(added.list, ['1920x1080', '1280x720', '2560x1440'])
  assert.deepEqual(list, ['1920x1080', '1280x720'], 'the old array is not touched')
  assert.deepEqual(addEntry('resolution', list, '1920X1080'), { list, error: 'exists' })
  assert.deepEqual(addEntry('resolution', list, '100x100'), { list, error: 'range' })
  assert.deepEqual(addEntry('resolution', list, 'x'), { list, error: 'invalid' })
  assert.deepEqual(addEntry('rate', ['60'], '60.0'), { list: ['60'], error: 'exists' })
  const full = Array.from({ length: MAX_ENTRIES }, (_, i) => String(i + 1))
  assert.equal(addEntry('rate', full, '300').error, 'full')
})

test('editing keeps the position; the same value written back is no error; a clash with another entry is', () => {
  const list = ['60', '120', '144']
  assert.deepEqual(editEntry('rate', list, 1, '119.88').list, ['60', '119.88', '144'])
  assert.deepEqual(editEntry('rate', list, 1, '120.0'), { list, value: '120' })
  assert.equal(editEntry('rate', list, 1, '144').error, 'exists')
  assert.equal(editEntry('rate', list, 1, '999').error, 'range')
  assert.equal(editEntry('rate', list, 7, '90').error, 'invalid')
})

test('removing: the last entry stays', () => {
  assert.deepEqual(removeEntry(['60', '120'], 0).list, ['120'])
  assert.deepEqual(removeEntry(['60'], 0), { list: ['60'], error: 'minOne' })
})

test('moving: one place up or down, nothing at the ends, the order is the order written', () => {
  const list = ['a', 'b', 'c']
  assert.deepEqual(moveEntry(list, 2, -1), ['a', 'c', 'b'])
  assert.deepEqual(moveEntry(list, 0, 1), ['b', 'a', 'c'])
  assert.equal(moveEntry(list, 0, -1), list)
  assert.equal(moveEntry(list, 2, 1), list)
})

test('sorting is a request: by pixels for resolutions, by value for rates', () => {
  assert.deepEqual(sortList('resolution', ['3840x2160', '1280x720', '1920x1080']), ['1280x720', '1920x1080', '3840x2160'])
  assert.deepEqual(sortList('rate', ['144', '59.94', '60', '120']), ['59.94', '60', '120', '144'])
})

test('a loaded list keeps its order and what the person already has, even outside the editor ranges', () => {
  assert.deepEqual(cleanList('resolution', ['2560x1440', '1920x1080', '2560x1440', '200x100']), ['2560x1440', '1920x1080', '200x100'])
  assert.deepEqual(cleanList('rate', ['120', '60', '60.0', '30']), ['120', '60', '30'])
  assert.deepEqual(cleanList('rate', [60, 144]), ['60', '144'])
})

test('the driver nodes round trip: list -> <resolution> nodes -> list, in order', () => {
  const list = ['2560x1440', '800x600', '1920x1080']
  const nodes = toResolutionNodes(list)
  assert.deepEqual(nodes, [{ width: 2560, height: 1440 }, { width: 800, height: 600 }, { width: 1920, height: 1080 }])
  assert.deepEqual(fromResolutionNodes(nodes), list)
  assert.deepEqual(fromResolutionNodes([{ width: 0, height: 1080 }, null, { width: 1920, height: 1080 }]), ['1920x1080'])
})

test('the page draws both lists with the editor, keeps the order and names the product', () => {
  const page = readFileSync(new URL('../components/VddSettings.vue', import.meta.url), 'utf8')
  assert.match(page, /<VddModeList v-model="resolutionList" kind="resolution"/)
  assert.match(page, /<VddModeList v-model="rateList" kind="rate"/)
  assert.doesNotMatch(page, /sortedResolutionOptions|sortedRefreshRateOptions|useEditableOptionField/, "the page never sorts the lists on its own")
  assert.match(page, /resolution: resolutionList\.value\.map/)
  assert.match(page, /g_refresh_rate: \[\.\.\.rateList\.value\]/)
  const zh = readFileSync(new URL('../desktop/i18n/zh.js', import.meta.url), 'utf8')
  const en = readFileSync(new URL('../desktop/i18n/en.js', import.meta.url), 'utf8')
  assert.doesNotMatch(zh, /基地显示器/)
  assert.match(zh, /title: '杂鱼显示器设置'/)
  assert.match(en, /title: 'ZakoDisplay Settings'/)
})
