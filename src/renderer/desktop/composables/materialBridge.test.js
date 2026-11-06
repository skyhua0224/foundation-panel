import assert from 'node:assert/strict'
import test from 'node:test'
import { hexOf, parseColour } from './materialBridge.js'

test('parseColour reads rgb(), rgba() and hex', () => {
  assert.deepEqual(parseColour('rgb(10, 20, 30)'), [10, 20, 30])
  assert.deepEqual(parseColour('rgba(10 20 30 / 0.5)'), [10, 20, 30])
  assert.deepEqual(parseColour('#0af'), [0, 170, 255])
  assert.deepEqual(parseColour('#102030'), [16, 32, 48])
  assert.equal(parseColour('var(--x)'), null)
  assert.equal(parseColour(''), null)
})

test('hexOf pads each channel', () => {
  assert.equal(hexOf([0, 5, 255]), '#0005ff')
})
