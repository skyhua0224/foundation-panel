import { test } from 'node:test'
import assert from 'node:assert/strict'
import QRCode from 'qrcode'
import { qrSvg, qrSvgDataUrl } from './qrSvg.js'

test('the SVG of a QR matrix draws exactly the dark modules of the matrix, in a quiet zone of 2', () => {
  const modules = QRCode.create('https://127.0.0.1:57990/pair?code=1234', { errorCorrectionLevel: 'M' }).modules
  const svg = qrSvg(modules)
  const total = modules.size + 4
  assert.ok(svg.includes(`viewBox="0 0 ${total} ${total}"`))
  // every dark module is covered by a run of the path, and nothing else is
  const path = svg.match(/<path d="([^"]*)"/)[1]
  const cells = new Set()
  for (const m of path.matchAll(/M(\d+) (\d+)h(\d+)v1h-\d+z/g)) for (let i = 0; i < Number(m[3]); i += 1) cells.add(`${Number(m[2]) - 2},${Number(m[1]) + i - 2}`)
  let dark = 0
  for (let r = 0; r < modules.size; r += 1) for (let c = 0; c < modules.size; c += 1) { const on = !!modules.get(r, c); if (on) dark += 1; assert.equal(cells.has(`${r},${c}`), on, `${r},${c}`) }
  assert.equal(cells.size, dark)
})

test('the image address is a UTF-8 SVG data URL that decodes to the same picture', () => {
  const modules = QRCode.create('abc', { errorCorrectionLevel: 'M' }).modules
  const url = qrSvgDataUrl(modules)
  assert.ok(url.startsWith('data:image/svg+xml;charset=utf-8,'))
  assert.equal(decodeURIComponent(url.slice('data:image/svg+xml;charset=utf-8,'.length)), qrSvg(modules))
})
