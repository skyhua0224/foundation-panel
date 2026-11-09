import test from 'node:test'
import assert from 'node:assert/strict'
import { zh } from '../desktop/i18n/zh.js'
import { en } from '../desktop/i18n/en.js'
import { MICROPHONE_BACKENDS } from './deviceHubStatus.js'

const STATES = ['unknown', 'disabled', 'unsupported', 'faulted', 'missing', 'waiting', 'active', 'starting', 'idle', 'capturing', 'streaming', 'stopping']
const CODES = [
  'MIC_BACKEND_DISABLED', 'MIC_TEST_BUSY', 'MIC_TEST_COM_FAILED', 'MIC_TEST_DEVICE_UNAVAILABLE', 'MIC_TEST_UNSUPPORTED',
  'MIC_TEST_WRITE_FAILED', 'MIC_USBIP_COMPONENT_UNAVAILABLE', 'MIC_USBIP_CREATE_FAILED', 'MIC_USBIP_DEVICE_BUSY',
  'MIC_USBIP_START_TIMEOUT', 'MIC_USBIP_TRANSPORT_LOST',
]
const LIVE_KEYS = ['howToTest', 'testLive', 'level', 'levelActive', 'levelSilent']

for (const [name, locale] of [['zh', zh], ['en', en]]) {
  test(`${name}: the microphone tab has every state, backend and host error code`, () => {
    const text = locale.deviceHub.microphone
    for (const state of STATES) {
      assert.ok(text.statusLabels[state], `label ${state}`)
      assert.ok(text.summaries[state], `summary ${state}`)
    }
    for (const backend of MICROPHONE_BACKENDS) {
      assert.ok(text.backends[backend], backend)
      assert.ok(text.backendHints[backend], `${backend} hint`)
    }
    for (const code of [...CODES, 'unknown']) assert.ok(text.errors[code], code)
    for (const key of LIVE_KEYS) assert.ok(typeof text[key] === 'string' && text[key], key)
  })

  test(`${name}: one line tells the user how to test, idle and while streaming`, () => {
    const how = locale.deviceHub.microphone.howToTest
    // English sentence case differs from the label capitalization, so compare insensitively.
    const fold = (value) => value.toLowerCase()
    assert.ok(fold(how).includes(fold(locale.deviceHub.microphone.test)), 'names the idle test')
    assert.ok(fold(how).includes(fold(locale.deviceHub.microphone.level)), 'names the live level')
  })

  test(`${name}: the microphone wording no longer calls the backend experimental`, () => {
    // Keys such as usbip_experimental are the host's value names; only the sentences count.
    const sentences = (node) => (typeof node === 'string' ? [node] : Object.values(node).flatMap(sentences))
    const all = sentences(locale.deviceHub.microphone).join('\n')
    assert.doesNotMatch(all, /experimental|实验|Virtual Device Host|prototype|原型|Sidecar|DualSense/i)
  })
}
