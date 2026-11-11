import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { undeclaredIdentifiers } from 'foundation-ui/testing/scope-check'

// rv-ui-tile-round F1: the overview called say(...) that it had never declared or imported, and every test passed, because
// such a branch only fails at run time in the hand of a user. The whole script of the overview is checked: every identifier
// it uses must be declared, imported or given somewhere in the file, so a helper that disappears from the file fails here.
const source = readFileSync(new URL('../components/PanelHome.vue', import.meta.url), 'utf8')

test('PanelHome.vue uses only names it declares, imports or is given (say is one of them)', () => {
  assert.deepEqual(undeclaredIdentifiers(source, { filename: 'PanelHome.vue' }), [])
})

test('losing say again is what this test is for', () => {
  const without = source.replace(/const say = .+\n/, '')
  assert.ok(undeclaredIdentifiers(without, { filename: 'PanelHome.vue' }).some((x) => x.name === 'say'), 'an undeclared say is reported')
})

// home-streaming-fix 2026-11-11: the overview gave MHomeDash neither the host configuration nor the language, so the pinned 自动 FEC tile showed its default state in English (「Off」 under a Chinese
// title) whatever the host had set. The pinned setting tiles read `config` and speak `locale`; losing either prop brings the wrong state back.
test("the overview gives the dashboard the host's configuration and the language (the pinned setting tiles read them)", () => {
  const tag = /<MHomeDash\b[^>]*>/s.exec(source)?.[0] || ''
  assert.match(tag, /:config="state\.config"/)
  assert.match(tag, /:locale="locale"/)
})
