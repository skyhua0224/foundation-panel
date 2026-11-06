import assert from 'node:assert/strict'
import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

// Every page title sits at one position (foundation-ui layout.css: one band, one row height, one size). The panel only places pages in the detail pane; a rule of its own
// that moves, resizes or re-pads the title row of one kind of page is how the titles of 手柄 and USB 转发 came to sit in different places.
const root = new URL('../', import.meta.url).pathname
const files = []
const walk = (dir) => {
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, e.name)
    if (e.isDirectory()) walk(p)
    else if (/\.(vue|css|less)$/.test(e.name)) files.push(p)
  }
}
walk(root)

test('no stylesheet or style block of the panel styles the page title row', () => {
  assert.ok(files.length > 20)
  const hits = []
  for (const f of files) {
    const text = readFileSync(f, 'utf8')
    for (const m of text.matchAll(/[^{}\n]*\.m-page(__title|__bar\b|--(sub|wide|full|cols)\b[^{}]*\.m-page__(title|bar))[^{}]*\{/g)) hits.push(`${f.slice(root.length)}: ${m[0].trim().slice(0, 100)}`)
  }
  assert.deepEqual(hits, [])
})

test('the panel hands its frame no margin or padding above the web page: the title of a framed page is where a panel title is', () => {
  const host = readFileSync(join(root, 'components/PanelWebHost.vue'), 'utf8')
  assert.match(host, /\.panel-web__frame \{ position: relative; flex: 1; min-height: 0; \}/)
  assert.doesNotMatch(host, /\.panel-web__frame \{[^}]*(padding|margin)/)
})

test('a framed page starts where a panel page starts: the injected stylesheet reserves no top padding, and the detail pane of the panel hides its scrollbar like the frame', () => {
  const inject = readFileSync(join(root, '../../src-tauri/inject-styles.css'), 'utf8')
  assert.match(inject, /html\[data-sunshine-navigation='unified'\] body \{[^}]*padding-top: 0 !important;/)
  const menu = readFileSync(join(root, 'components/SidebarMenu.vue'), 'utf8')
  assert.match(menu, /\.panel-body \.m-scene__pane--detail \{ scrollbar-width: none; \}/)
})
