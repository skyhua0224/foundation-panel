import assert from 'node:assert/strict'
import test from 'node:test'
import { isMaterialTextField, isTextEntryElement } from './useOsk.js'

test('Material text fields count as text entry', () => {
  assert.equal(isMaterialTextField({ localName: 'md-outlined-text-field' }), true)
  assert.equal(isMaterialTextField({ localName: 'md-filled-text-field' }), true)
  assert.equal(isMaterialTextField({ localName: 'button' }), false)
  assert.equal(isTextEntryElement({ localName: 'md-outlined-text-field', type: 'text' }), true)
  assert.equal(isTextEntryElement({ localName: 'md-outlined-text-field', type: 'number' }), true)
})

test('other elements are not text entry outside a browser', () => {
  assert.equal(isTextEntryElement(null), false)
  assert.equal(isTextEntryElement({ localName: 'div' }), false)
})
