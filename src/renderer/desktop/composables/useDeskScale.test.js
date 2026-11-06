import assert from 'node:assert/strict'
import test from 'node:test'
import { DESK_BASE_SCALE, deskScale } from './useDeskScale.js'

test('the sofa scale follows the text size of the theme editor around the Material default', () => {
  assert.equal(deskScale('16px'), DESK_BASE_SCALE)
  assert.equal(deskScale('18px'), 1.406)
  assert.equal(deskScale('12px'), 0.938)
})

test('a text size that is not a pixel size leaves the base scale', () => {
  for (const value of ['', '   ', 'large', undefined, null, '0px', '400px', 'NaN']) assert.equal(deskScale(value), DESK_BASE_SCALE, String(value))
})
