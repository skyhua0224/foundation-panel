import test from 'node:test'
import assert from 'node:assert/strict'
import { SPONSORS as ABOUT } from 'foundation-ui/about'
import { SPONSORS } from './sponsors.js'

test('sponsor targets are the team\'s own 爱发电 pages in team order, the same list as the about page', () => {
  assert.deepEqual(SPONSORS.map((s) => [s.label, s.url]), ABOUT.map((s) => [s.login, s.url]))
  assert.deepEqual(SPONSORS.map((s) => s.label), ['qiin2333', 'Yundi339', 'skyhua0224'])
  for (const s of SPONSORS) assert.match(s.url, /^https:\/\//)
})
