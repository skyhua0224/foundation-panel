import test from 'node:test'
import assert from 'node:assert/strict'
import { fileURLToPath } from 'node:url'
import { join } from 'node:path'
import { scanFile, listFiles } from 'foundation-ui/testing/copy-lint'
import { zh } from '../renderer/desktop/i18n/zh.js'
import { zh_TW } from '../renderer/desktop/i18n/zh_TW.js'
import { en } from '../renderer/desktop/i18n/en.js'
import { en_GB } from '../renderer/desktop/i18n/en_GB.js'
import { en_US } from '../renderer/desktop/i18n/en_US.js'

// docs/copy-style.zh.md (foundation-ui): the banned words and patterns of the interface text, in Chinese and English.
const root = fileURLToPath(new URL('../', import.meta.url))
const TEXT_FILES = [
  'i18n/locales/zh.js', 'i18n/locales/zh_TW.js', 'i18n/locales/en.js', 'i18n/locales/en_GB.js', 'i18n/locales/en_US.js',
  'renderer/desktop/i18n/zh.js', 'renderer/desktop/i18n/zh_TW.js', 'renderer/desktop/i18n/en.js', 'renderer/desktop/i18n/en_GB.js', 'renderer/desktop/i18n/en_US.js',
  'renderer/home/i18n.js',
].map((f) => join(root, f))

// The desktop pet 米塔 talks in a character voice on purpose: its sayings and the instruction that makes the model speak that way are not interface copy.
// Its status and error messages are, and are judged like every other text.
const norm = (s) => String(s).replace(/\\n/g, '\n').replace(/\\(['"`])/g, '$1').trim()
const persona = new Set()
const collect = (o) => {
  if (!o || typeof o !== 'object') return
  if (Array.isArray(o.defaultPhrases)) for (const s of o.defaultPhrases) persona.add(norm(s))
  if (typeof o.visionPrompt === 'string') persona.add(norm(o.visionPrompt))
  for (const v of Object.values(o)) collect(v)
}
for (const t of [zh, zh_TW, en, en_GB, en_US]) collect(t)
const personaText = (s) => persona.has(norm(s))

test('the pet sayings are found (so the persona exemption is a short, explicit list)', () => {
  assert.ok(persona.size >= 20 && persona.size < 80, String(persona.size))
})

test('no panel text breaks the copy rules', () => {
  const hits = []
  for (const file of TEXT_FILES) for (const h of scanFile(file, { allowText: personaText })) hits.push(`${file.replace(root, 'src/')}: [${h.rule}] ${h.text.slice(0, 120)}`)
  for (const file of listFiles(join(root, 'renderer'), ['.vue'])) for (const h of scanFile(file)) hits.push(`${file.replace(root, 'src/')}: [${h.rule}] ${h.text.slice(0, 120)}`)
  assert.deepEqual(hits, [], `copy rule violations:\n${hits.join('\n')}`)
})

test('the pet\'s status messages are plain: what happened, then what to do', () => {
  const find = (o, key) => { if (!o || typeof o !== 'object') return null; if (key in o) return o[key]; for (const x of Object.values(o)) { const r = find(x, key); if (r) return r } return null }
  assert.equal(find(zh, 'visionNotConfigured'), '米塔 AI 尚未配置，请在「米塔 AI」中填写 API Key。')
  assert.equal(find(en, 'visionNotConfigured'), 'Mita AI is not set up. Enter an API key in Mita AI.')
  assert.equal(find(zh, 'errors').notVlm, '该模型不支持图像识别，请换用支持视觉的模型。')
})
