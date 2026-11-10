import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import { sparxieLang, sparxieMore, sparxieTags } from './composables/sparxieTags.js'

test('a Sparxie application carries Sparxie, its region and its launch mode; any other application nothing', () => {
  assert.deepEqual(sparxieTags({ name: 'g', sparxie: { region: 'bili', mode: 'touch' } }, 'zh').map((t) => t.label), ['Sparxie', 'B 服', '触控'])
  assert.deepEqual(sparxieTags({ name: 'g', sparxie: { region: 'global', mode: 'pc' } }, 'en').map((t) => t.label), ['Sparxie', 'Global', 'Keyboard & gamepad'])
  assert.deepEqual(sparxieTags({ name: 'Steam' }, 'zh'), [])
  assert.equal(sparxieTags({ sparxie: { mode: 'touch' } }, 'zh').length, 2, 'no region chip while the region is not known')
})

test('without the component nothing of Sparxie is drawn, even for an application that has the block', () => {
  assert.deepEqual(sparxieTags({ sparxie: { region: 'cn', mode: 'touch' } }, 'zh', false), [])
})

test('Chinese for every Chinese locale, English for the rest', () => {
  assert.equal(sparxieLang('zh_TW'), 'zh'); assert.equal(sparxieLang('de'), 'en')
  assert.equal(sparxieMore('en_US'), 'More')
})

test('the big-screen tile draws the marks in one row under the name', () => {
  const grid = readFileSync(new URL('./components/AppGridView.vue', import.meta.url), 'utf8')
  assert.ok(grid.includes('<MChipRow v-if="spxTags(app).length"') && grid.includes('sparxieTags(app, locale.value, spx.installed)'))
})

test('the Sparxie chip carries the mark; the picture is the panel\'s own public asset', () => {
  assert.equal(sparxieTags({ sparxie: { region: 'cn', mode: 'touch' } }, 'zh')[0].mark, true)
  const grid = readFileSync(new URL('./components/AppGridView.vue', import.meta.url), 'utf8')
  assert.ok(grid.includes("'/sparxie/sparxie-mark-64.png'") && grid.includes('tag.mark ? { ...tag, image: SPARXIE_MARK }'))
})
