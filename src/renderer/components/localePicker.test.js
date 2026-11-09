import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { DESKTOP_LOCALES, LOCALE_NAMES } from '../desktop/i18n/merge.js'

const page = readFileSync(new URL('./ControlPanelPage.vue', import.meta.url), 'utf8')

test('the settings language list offers follow-system and all 20 endonyms', () => {
  assert.match(page, /DESKTOP_LOCALES\.map/)
  assert.equal(DESKTOP_LOCALES.length, 20)
  assert.equal(new Set(DESKTOP_LOCALES).size, 20)
  for (const code of DESKTOP_LOCALES) assert.ok(LOCALE_NAMES[code], code)
  assert.equal(LOCALE_NAMES.zh_TW, '繁體中文')
  assert.equal(LOCALE_NAMES.de, 'Deutsch')
  assert.equal(LOCALE_NAMES.ja, '日本語')
  assert.equal(LOCALE_NAMES.pt_BR, 'Português (Brasil)')
})
