import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'

// Tauri's drag script (tauri 2.x, src/window/scripts/drag.js) reads a bare data-tauri-drag-region as "only a click straight on this element drags". The gaps between the buttons
// of the caption bar belong to wrapper elements of the caption, so the bar says "deep": a click anywhere in it drags, except on a button or a link (those stay clickable).
const source = readFileSync(new URL('./SidebarMenu.vue', import.meta.url), 'utf8')

test('the caption bar is a deep drag region', () => {
  assert.match(source, /<MCaption bem="panel-caption" data-tauri-drag-region="deep" @pointerdown="onTouchWindowDragStart">/)
})

test('no caption control opts out of being clickable by carrying the drag attribute', () => {
  const buttons = source.match(/<button[^>]*>/g) || []
  assert.ok(buttons.length >= 4)
  for (const tag of buttons) assert.doesNotMatch(tag, /data-tauri-drag-region/, tag)
})
