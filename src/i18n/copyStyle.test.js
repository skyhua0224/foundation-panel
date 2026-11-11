import test from 'node:test'
import assert from 'node:assert/strict'
import { fileURLToPath } from 'node:url'
import { join } from 'node:path'
import { scanFile, listFiles } from 'foundation-ui/testing/copy-lint'
import { zh } from '../renderer/desktop/i18n/zh.js'
import { en } from '../renderer/desktop/i18n/en.js'

// docs/copy-style.zh.md (foundation-ui): the explaining voice of the interface text, in Chinese and English. The brand voice (the pet, the tray, the greetings) is deliberate and is not checked.
const root = fileURLToPath(new URL('../', import.meta.url))
const TEXT_FILES = [
  'i18n/locales/zh.js', 'i18n/locales/zh_TW.js', 'i18n/locales/en.js', 'i18n/locales/en_GB.js', 'i18n/locales/en_US.js',
  'renderer/desktop/i18n/zh.js', 'renderer/desktop/i18n/zh_TW.js', 'renderer/desktop/i18n/en.js', 'renderer/desktop/i18n/en_GB.js', 'renderer/desktop/i18n/en_US.js',
  'renderer/home/i18n.js',
].map((f) => join(root, f))

test('no panel text breaks the copy rules', () => {
  const hits = []
  for (const file of TEXT_FILES) for (const h of scanFile(file, {})) hits.push(`${file.replace(root, 'src/')}: [${h.rule}] ${h.text.slice(0, 120)}`)
  for (const file of listFiles(join(root, 'renderer'), ['.vue'])) for (const h of scanFile(file)) hits.push(`${file.replace(root, 'src/')}: [${h.rule}] ${h.text.slice(0, 120)}`)
  assert.deepEqual(hits, [], `copy rule violations:\n${hits.join('\n')}`)
})

test('the pet keeps its character voice: the sayings and the empty states are the original lines', () => {
  const find = (o, key) => { if (!o || typeof o !== 'object') return null; if (key in o) return o[key]; for (const x of Object.values(o)) { const r = find(x, key); if (r) return r } return null }
  assert.equal(find(zh, 'noStream'), '杂鱼~ 没有开始串流还在调码率呢')
  assert.equal(find(zh, 'visionNotConfigured'), '还没配好米塔 AI 哦，去「米塔 AI」填个 Key 吧～')
  assert.equal(find(en, 'visionNotConfigured'), 'Mita AI is not set up yet, fill in a Key in Mita AI~')
})
