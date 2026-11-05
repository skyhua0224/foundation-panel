import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

// The old interface of the panel (the pixel fonts, the skewed text, the plum and cream 'morandi' palette, the old failure page) is gone and must not come back: nothing the person can see may carry it.
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..')
const OLD = [/PixelMplus/i, /YouYuan/i, /Fusion ?Pixel/i, /morandi/i, /skew\(/, /#2d2628/i, /#3d3235/i, /#d4a5a5/i, /#e6d5b8/i]
const EXT = /\.(vue|js|mjs|less|css|html|rs)$/
const SKIP_DIR = new Set(['node_modules', 'dist', 'target', '_harness', '.git', 'gen'])
function* walk(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (SKIP_DIR.has(entry.name)) continue
    const full = path.join(dir, entry.name)
    if (entry.isDirectory()) yield* walk(full)
    else if (EXT.test(entry.name) && !/\.test\.(js|mjs)$/.test(entry.name)) yield full
  }
}

test('no file the panel is built from carries the old interface', () => {
  const hits = []
  for (const base of ['src', 'src-tauri/src']) {
    for (const file of walk(path.join(root, base))) {
      const text = fs.readFileSync(file, 'utf8')
      for (const re of OLD) if (re.test(text)) hits.push(`${path.relative(root, file)}: ${re}`)
    }
  }
  for (const file of ['src-tauri/error-404.html', 'src-tauri/inject-styles.css']) {
    const text = fs.readFileSync(path.join(root, file), 'utf8')
    for (const re of OLD) if (re.test(text)) hits.push(`${file}: ${re}`)
  }
  assert.deepEqual(hits, [])
})

test('the pixel fonts and the old style files are not shipped', () => {
  for (const gone of ['src/renderer/public/fonts/PixelMplus12-Bold.woff2', 'src/renderer/public/fonts/fusion-pixel-12px-proportional-zh_hans.woff2', 'src/renderer/styles/pixel-language.less', 'src/renderer/styles/ControllersHub.less', 'src/renderer/styles/SidebarMenu.less', 'src/renderer/styles/element-theme.less', 'src/renderer/styles/dialog.less', 'src/renderer/styles/theme.less', 'src/renderer/log-console']) {
    assert.equal(fs.existsSync(path.join(root, gone)), false, `${gone} is back`)
  }
})

test('the first frame is the Material 3 surface the window reports, not the plum gradient', () => {
  const html = fs.readFileSync(path.join(root, 'src/renderer/sunshine-frame.html'), 'utf8')
  assert.match(html, /--boot-background: #131318/)
  assert.doesNotMatch(html, /linear-gradient/)
  const rust = fs.readFileSync(path.join(root, 'src-tauri/src/windows/surface.rs'), 'utf8')
  assert.match(rust, /const FIRST_RUN: \(u8, u8, u8\) = \(0x13, 0x13, 0x18\)/)
})
