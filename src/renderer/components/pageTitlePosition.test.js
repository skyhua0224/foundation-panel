import assert from 'node:assert/strict'
import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

// Every page has one left edge (foundation-ui layout.css): the title and the content start at the pane's inset, whatever the width of the page. The panel only places pages in
// the detail pane; a rule of its own that moves, resizes or re-pads the title row of one kind of page, or centres a page by its own margin, is how the titles of 手柄 and
// USB 转发 came to sit in different places and the content at another x than the title.
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

test('no band, offset or centred column of the old rule is left in the panel', () => {
  for (const f of files) {
    const text = readFileSync(f, 'utf8')
    assert.doesNotMatch(text, /--m-title-(band|off|gutter)/, `${f.slice(root.length)} uses a variable of the removed rule`)
  }
})

test('title left equals content left: the pages of the panel that sit in the detail pane start at the inset, not at the middle of the pane', () => {
  const compat = readFileSync(join(root, 'styles/m3e-compat.less'), 'utf8')
  assert.doesNotMatch(compat, /margin: 0 auto/, 'a legacy page centres itself')
  const ai = readFileSync(join(root, 'components/AiAssistant.css'), 'utf8')
  // the chat's header, conversation and input take the inset of the pages (--m-gutter, the same container rules), and nothing is centred by max()/calc() around --m-column
  assert.match(ai, /\.ai-shell \{ --m-gutter: 32px;/)
  assert.match(ai, /@container \(max-width: 700px\) \{ \.ai-shell \{ --m-gutter: 16px; \} \}/)
  assert.match(ai, /\.ai-head \{[^}]*padding: 16px var\(--m-gutter\) 8px;/)
  assert.match(ai, /\.ai-col \{[^}]*margin: 0; padding: 8px var\(--m-gutter\) 16px;/)
  assert.match(ai, /\.ai-dock \{[^}]*padding: 8px var\(--m-gutter\) 16px;/)
  assert.match(ai, /\.ai-composer \{ max-width: calc\(var\(--m-column, 960px\) - 2 \* var\(--m-gutter\)\); margin: 0;/)
  assert.doesNotMatch(ai, /\(100% - var\(--m-column/)
  const host = readFileSync(join(root, 'components/PanelWebHost.vue'), 'utf8')
  assert.match(host, /\.panel-web__cover > \.m-pageload \{ width: min\(100%, 960px\); margin-inline: 0; \}/)
  assert.match(host, /\.panel-web \{ --m-gutter: 32px;/)
  assert.match(host, /\.panel-web__bar \{ padding: 8px var\(--m-gutter\); flex: none; \}/)
  assert.match(host, /@container \(max-width: 560px\) \{ \.panel-web__bar, \.panel-web__cover \{ --m-gutter: 12px; \} \}/)
})
