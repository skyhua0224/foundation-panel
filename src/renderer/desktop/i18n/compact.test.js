// The compact form of a language file stands for the same language: for every language, the table a person sees (the merge with its
// fallbacks) is the same whether it is made from the file as written or from its compact form.
import assert from 'node:assert/strict'
import test from 'node:test'
import { compactLocale, expandLocale } from './compact.js'
import { coversTexts, mergeLocale } from './merge.js'
import { en } from './en.js'
import { zh } from './zh.js'

const codes = ['zh_TW', 'en_GB', 'en_US', 'de', 'fr', 'es', 'it', 'ja', 'ko', 'ru', 'pt_BR', 'pt', 'bg', 'cs', 'pl', 'sv', 'tr', 'uk']

for (const code of codes) {
  test(`${code}: the merged table is the same from the compact form`, async () => {
    const table = (await import(`./${code}.js`))[code]
    assert.ok(coversTexts(en.deviceHub.vbComponent, table.deviceHub?.vbComponent), `${code}: VB-CABLE text is incomplete`)
    const { values, extras } = compactLocale(table, en, { fallbacks: code === 'zh_TW' ? [zh, en] : [en] })
    const back = expandLocale(JSON.parse(JSON.stringify(values)), JSON.parse(JSON.stringify(extras)))
    assert.deepEqual(mergeLocale(en, zh, code, back), mergeLocale(en, zh, code, table))
  })
}

test('Chinese VB-CABLE component text covers the English source', () => {
  assert.ok(coversTexts(en.deviceHub.vbComponent, zh.deviceHub.vbComponent))
})

test('a table without a key gives the fallback, an empty text counts as none, and a key English lacks is kept', () => {
  const english = { a: 'one', b: { c: 'two', d: ['x', 'y'] } }
  const table = { a: 'eins', b: { c: '', d: ['x2', 'y'] }, extra: { deep: 'nur hier' } }
  const compact = compactLocale(table, english)
  assert.deepEqual(compact.values, ['eins', '', 'x2', ''], 'the unchanged "y" and the empty text are left out')
  assert.deepEqual(compact.extras, [[['extra', 'deep'], 'nur hier']])
})

test('English (UK) is a few kilobytes: nearly every text is English', async () => {
  const { en_GB } = await import('./en_GB.js')
  const { values } = compactLocale(en_GB, en)
  assert.ok(values.filter(Boolean).length < 200, 'only the spellings that differ remain')
})
