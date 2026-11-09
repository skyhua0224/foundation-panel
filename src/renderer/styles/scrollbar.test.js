// The Panel has no scrollbar rule of its own: foundation-ui's css/scrollbar.css draws every scrollbar (a Panel rule with scrollbar-width: thin / scrollbar-color, or its own ::-webkit-scrollbar, would
// switch the shared pill scrollbar off or draw a second one). A strip that hides its bar on purpose (`scrollbar-width: none`) is allowed. The overlay tool window (NrOverlayTool) keeps the accent colour of
// the overlay it draws over a game.
import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const walk = (dir) => fs.readdirSync(dir, { withFileTypes: true }).flatMap((e) => (e.isDirectory() ? (e.name === 'node_modules' ? [] : walk(path.join(dir, e.name))) : [path.join(dir, e.name)]))
test('no Panel stylesheet draws a scrollbar of its own', () => {
  const bad = []
  for (const f of walk(root).filter((x) => /\.(less|css|vue)$/.test(x) && !/NrOverlayTool\.vue$/.test(x))) {
    const text = fs.readFileSync(f, 'utf8')
    for (const m of text.matchAll(/::-webkit-scrollbar(?![\w-]*\s*\{\s*display:\s*none)|scrollbar-width:\s*(thin|auto)|scrollbar-color:/g)) bad.push(`${path.relative(root, f)}: ${m[0]}`)
  }
  assert.deepEqual(bad, [])
})

test('a framed page: only the page itself (document, detail pane) has no scrollbar; every other scroll container keeps the shared one', () => {
  const inject = fs.readFileSync(path.join(root, '../../src-tauri/inject-styles.css'), 'utf8')
  assert.match(inject, /html, body, \.m-scene__pane \{\s*scrollbar-width: none;\s*\}/)
  assert.doesNotMatch(inject, /\*\s*\{\s*scrollbar-width: none/)
  assert.doesNotMatch(inject, /^::-webkit-scrollbar/m)
})
