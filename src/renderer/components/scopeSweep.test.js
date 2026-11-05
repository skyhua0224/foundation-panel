import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { undeclaredIdentifiers } from 'foundation-ui/testing/scope-check'

for (const name of ['AiAssistant.vue', 'VddSettings.vue']) {
  test(`${name} uses only names it declares, imports or is given`, () => {
    const source = readFileSync(new URL(`./${name}`, import.meta.url), 'utf8')
    assert.deepEqual(undeclaredIdentifiers(source, { filename: name }), [])
  })
}
