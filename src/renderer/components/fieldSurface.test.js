import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'

// The rule of a text field lives in foundation-ui (src/css/fields.css, test/fieldsurface.test.js there); this pins that the panel's own fields inherit it:
// every md-outlined-text-field of the panel carries the container fill, and no page paints a field's outline itself.
const root = new URL('..', import.meta.url).pathname
const vueFiles = (dir) => readdirSync(dir).flatMap((n) => { const p = join(dir, n); return statSync(p).isDirectory() ? vueFiles(p) : p.endsWith('.vue') ? [p] : [] })
const files = vueFiles(root)
const m3e = (f) => readFileSync(new URL(`../../../node_modules/foundation-ui/src/${f}`, import.meta.url), 'utf8')

test('every md-outlined-text-field of the panel has the container fill', () => {
  let n = 0
  for (const f of files) {
    const src = readFileSync(f, 'utf8')
    for (const t of src.matchAll(/<md-outlined-text-field\b[\s\S]*?<\/md-outlined-text-field>/g)) { n++; assert.match(t[0], /<span slot="container" class="m-field__fill"/, `${f} draws a field without the container fill`) }
    assert.equal((src.match(/<md-outlined-text-field\b/g) || []).length, (src.match(/<\/md-outlined-text-field>/g) || []).length, `${f}: a field is not closed`)
  }
  assert.ok(n >= 5, 'the USB and RTSS fields are there')
})

test('no page of the panel paints a field outline or shape of its own', () => {
  for (const f of files) assert.doesNotMatch(readFileSync(f, 'utf8'), /--md-outlined-text-field-(outline-color|container-shape)/, f)
})

test('the library rule the panel inherits is there: the fill, the 3 : 1 edge', () => {
  const css = m3e('css/fields.css')
  assert.match(css, /--m-field-fill: var\(--md-sys-color-surface-container-highest\)/)
  assert.match(css, /--m-field-outline: var\(--md-sys-color-outline\)/)
})

test('the AI composer is a container with the field edge and a primary focus edge', () => {
  const css = readFileSync(new URL('./AiAssistant.css', import.meta.url), 'utf8')
  assert.match(css, /\.ai-composer \{[^}]*box-shadow: inset 0 0 0 1px var\(--m-field-outline\)/)
  assert.match(css, /\.ai-composer:focus-within \{ box-shadow: inset 0 0 0 2px var\(--md-sys-color-primary\)/)
})
