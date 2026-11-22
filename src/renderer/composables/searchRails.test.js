import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'

const shell = readFileSync(new URL('../components/SidebarMenu.vue', import.meta.url), 'utf8')

test('the rail of the window is a search target: its entries are docs of the search and choosing one opens the rail', () => {
  // 音频, 显示与视频 ... are what the person sees in the navigation bar; the search finds them by their names and by the synonyms of foundation-ui
  assert.match(shell, /\.\.\.rail\.value\.filter\(\(r\) => \['overview', 'stream', 'config', 'input', 'audio', 'display', 'system'\]\.includes\(r\.id\)\)\.map\(\(r\) => \(\{ id: `rail-\$\{r\.id\}`.*?railId: r\.id/)
  assert.match(shell, /if \(doc\.type === 'page' && doc\.entry\?\.railId\) onRail\(\{ id: doc\.entry\.railId \}\)/)
  // the rails that are pages of their own are not listed twice
  for (const id of ['assistant', 'settings', 'help', 'developer']) assert.doesNotMatch(shell, new RegExp(`includes\\(r\\.id\\)\\)\\.map[^\\n]*'${id}'`))
})
