import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import { ownBackBar, panelBackMessage, readFrameBack } from './frameBack.js'

const host = readFileSync(new URL('../components/PanelWebHost.vue', import.meta.url), 'utf8')

test('the panel tells the frame whether Back applies, as a plain boolean', () => {
  assert.deepEqual(panelBackMessage(true), { type: 'foundation-panel-back', show: true })
  assert.deepEqual(panelBackMessage(false), { type: 'foundation-panel-back', show: false })
  assert.deepEqual(panelBackMessage(undefined), { type: 'foundation-panel-back', show: false })
  assert.equal(panelBackMessage('yes').show, false)
})

test('what the frame says about Back is read strictly: a report, a press, a request, nothing else', () => {
  assert.deepEqual(readFrameBack({ type: 'foundation-back-inline', drawn: true }), { kind: 'inline', drawn: true })
  assert.deepEqual(readFrameBack({ type: 'foundation-back-inline', drawn: false }), { kind: 'inline', drawn: false })
  assert.equal(readFrameBack({ type: 'foundation-back-inline', drawn: 'true' }), null)
  assert.equal(readFrameBack({ type: 'foundation-back-inline' }), null)
  assert.deepEqual(readFrameBack({ type: 'foundation-back-click' }), { kind: 'click' })
  assert.deepEqual(readFrameBack({ type: 'foundation-back-request' }), { kind: 'request' })
  for (const bad of [null, undefined, 'foundation-back-click', 3, {}, { type: 'panel-painted' }]) assert.equal(readFrameBack(bad), null)
})

test('the panel keeps its own bar unless the page drew the arrow: an older Web never leaves a pair without a Back', () => {
  assert.equal(ownBackBar({ canBack: true, sub: false, inline: false }), true, 'no report (older Web, or a page without a title row): the panel bar')
  assert.equal(ownBackBar({ canBack: true, sub: false, inline: true }), false, 'the page drew the arrow: no second row')
  assert.equal(ownBackBar({ canBack: false, sub: false, inline: false }), false, 'wide window: no Back, as before')
  assert.equal(ownBackBar({ canBack: true, sub: true, inline: false }), false, 'a sub-page has its own back bubble, as before')
  assert.equal(ownBackBar({ canBack: undefined, sub: false, inline: false }), false, 'no scene, no bar')
})

test('PanelWebHost: bar only through ownBackBar, reports from the frame\'s own window only, a press goes to the scene\'s Back', () => {
  assert.match(host, /v-if="ownBackBar\(\{ canBack: scene\?\.canBack\.value, sub, inline: inlineBack \}\)" class="panel-web__bar"/)
  assert.match(host, /if \(event\.source !== frameEl\(\)\?\.contentWindow\) return/)
  assert.match(host, /back\.kind === 'inline'\) \{ inlineBack\.value = back\.drawn; pushBack\(\) \}/)
  assert.match(host, /back\.kind === 'click' && scene\?\.canBack\.value === true && !props\.sub\) scene\.back\(\)/)
  assert.match(host, /watch\(\(\) => scene\?\.canBack\.value === true, pushBack\)/, 'the frame is told when Back starts or stops applying')
  assert.match(host, /postMessage\(panelBackMessage\(scene\?\.canBack\.value === true\), url\.origin\)/, 'sent to the frame\'s origin, never *')
  assert.match(host, /const inlineBack = ref\(false\)/, 'nothing is inline until the frame says so')
})
