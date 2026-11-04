import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'
import { zh } from '../desktop/i18n/zh.js'

// A template that interpolates a whole group of texts (an object) prints it as JSON on the screen, as the 桌宠 consent dialog once did.
const groups = []
const walk = (node, path) => {
  for (const [key, value] of Object.entries(node || {})) {
    if (value && typeof value === 'object' && !Array.isArray(value)) {
      const leaves = Object.values(value).every((v) => typeof v === 'string')
      if (leaves) groups.push(`${path}.${key}`)
      walk(value, `${path}.${key}`)
    }
  }
}
walk(zh, 't')

const vueFiles = (dir) => readdirSync(dir).flatMap((name) => {
  const p = join(dir, name)
  return statSync(p).isDirectory() ? vueFiles(p) : p.endsWith('.vue') ? [p] : []
})

test('no template prints a whole group of texts', () => {
  const found = []
  for (const file of vueFiles('src/renderer')) {
    const src = readFileSync(file, 'utf8')
    for (const g of groups) {
      const re = new RegExp(`\\{\\{\\s*${g.replace(/\./g, '\\.')}\\s*\\}\\}`)
      if (re.test(src)) found.push(`${file}: ${g}`)
    }
  }
  assert.deepEqual(found, [])
})
