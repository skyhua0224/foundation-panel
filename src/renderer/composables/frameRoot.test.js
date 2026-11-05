import test from 'node:test'
import assert from 'node:assert/strict'
import { rootSnapshot } from './frameRoot.js'

const root = (style, classes, theme) => ({
  style: Object.assign(Object.keys(style)[Symbol.iterator] ? { [Symbol.iterator]: () => Object.keys(style)[Symbol.iterator](), getPropertyValue: (n) => style[n] } : {}),
  classList: { contains: (c) => classes.includes(c) },
  getAttribute: (n) => (n === 'data-bs-theme' ? theme : null),
  ownerDocument: { getElementById: (id) => (id === 'foundation-ui-theme' ? { textContent: ':root {\n--md-sys-color-primary: #123456;\n}' } : null) },
})

test('the snapshot carries the colours and the scheme, not the picture', () => {
  const snap = rootSnapshot(root({ '--md-sys-color-primary': '#123456', '--m-scrim': '0.4', '--m-wallpaper': 'url(data:image/png;base64,AAAA)', '--m-wp-blur': '32px', color: 'red' }, ['m-has-wallpaper', 'other'], 'dark'))
  assert.deepEqual(snap.props, { '--md-sys-color-primary': '#123456', '--m-scrim': '0.4' })
  assert.deepEqual(snap.classes, ['m-has-wallpaper'])
  assert.equal(snap.theme, 'dark')
  assert.equal(snap.type, 'foundation-root')
  assert.match(snap.themeCss, /--md-sys-color-primary/)
})
