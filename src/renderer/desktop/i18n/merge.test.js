import test from 'node:test'
import assert from 'node:assert/strict'
import { DESKTOP_LOCALES, LOCALE_NAMES, documentLang, mergeLocale, normalizeDesktopLocale } from './merge.js'

test('normalizeDesktopLocale accepts the 20 codes and keeps zh_TW', () => {
  for (const code of DESKTOP_LOCALES) assert.equal(normalizeDesktopLocale(code), code)
  assert.equal(normalizeDesktopLocale('zh-TW'), 'zh_TW')
  assert.equal(normalizeDesktopLocale('zh-Hant'), 'zh_TW')
  assert.equal(normalizeDesktopLocale('zh-CN'), 'zh')
  assert.equal(normalizeDesktopLocale('en-GB'), 'en_GB')
  assert.equal(normalizeDesktopLocale('pt-BR'), 'pt_BR')
  assert.equal(normalizeDesktopLocale('de-DE'), 'de')
  assert.equal(normalizeDesktopLocale('ja-JP'), 'ja')
  assert.equal(normalizeDesktopLocale('klingon'), 'en')
  assert.equal(DESKTOP_LOCALES.length, 20)
  for (const code of DESKTOP_LOCALES) assert.ok(LOCALE_NAMES[code])
})

test('mergeLocale falls back per key and never returns an empty stand-in', () => {
  const en = { a: 'A', nest: { b: 'B', c: 'C' }, list: ['one', 'two'] }
  const zh = { a: '甲', nest: { b: '乙' }, list: ['一'] }
  const tw = { a: '臺' }
  const de = { nest: { c: 'Ce' } }
  assert.equal(mergeLocale(en, zh, 'zh', null).a, '甲')
  assert.equal(mergeLocale(en, zh, 'zh', null).nest.c, 'C')
  const twTable = mergeLocale(en, zh, 'zh_TW', tw)
  assert.equal(twTable.a, '臺')
  assert.equal(twTable.nest.b, '乙')
  assert.equal(twTable.nest.c, 'C')
  assert.equal(twTable.list[1], 'two')
  const deTable = mergeLocale(en, zh, 'de', de)
  assert.equal(deTable.a, 'A')
  assert.equal(deTable.nest.c, 'Ce')
  assert.equal(mergeLocale(en, zh, 'ja', {}), en)
  assert.equal(documentLang('zh'), 'zh-CN')
  assert.equal(documentLang('zh_TW'), 'zh-TW')
  assert.equal(documentLang('en_GB'), 'en-GB')
  assert.equal(documentLang('pt_BR'), 'pt-BR')
})
