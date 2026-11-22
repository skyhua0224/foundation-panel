import test from 'node:test'
import assert from 'node:assert/strict'
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

test('Panel selects the locked dependency by default and a checkout only when requested', async () => {
  const previous = process.env.FOUNDATION_UI_DIR
  const dir = mkdtempSync(join(tmpdir(), 'panel-ui-test-'))
  try {
    mkdirSync(join(dir, 'src/vue'), { recursive: true })
    writeFileSync(join(dir, 'package.json'), JSON.stringify({ exports: {
      '.': './src/index.js', './vue': './src/vue/index.js',
      './pending': './src/pending/index.js', './settings/*': './src/settings/*',
    } }))
    delete process.env.FOUNDATION_UI_DIR
    const locked = (await import('./vite.config.js?locked')).default()
    const lockedVue = locked.resolve.alias.find(alias => alias.find.test('foundation-ui/vue'))
    const installedRoot = resolve(dirname(fileURLToPath(import.meta.resolve('foundation-ui'))), '..')
    assert.equal(lockedVue.replacement, join(installedRoot, 'src/vue/index.js'))

    process.env.FOUNDATION_UI_DIR = dir
    const local = (await import('./vite.config.js?local')).default()
    for (const [name, expected] of [
      ['foundation-ui', 'src/index.js'],
      ['foundation-ui/vue', 'src/vue/index.js'],
      ['foundation-ui/pending', 'src/pending/index.js'],
      ['foundation-ui/settings/example.js', 'src/settings/example.js'],
    ]) {
      const alias = local.resolve.alias.find(item => item.find.test(name))
      assert.ok(alias, name)
      assert.equal(resolve(name.replace(alias.find, alias.replacement)), join(dir, expected))
    }
    assert.ok(local.server.fs.allow.includes(dir))
    process.env.FOUNDATION_UI_DIR = join(dir, 'missing')
    await assert.rejects(import('./vite.config.js?missing'), { code: 'ENOENT' })
  } finally {
    if (previous === undefined) delete process.env.FOUNDATION_UI_DIR
    else process.env.FOUNDATION_UI_DIR = previous
    rmSync(dir, { recursive: true, force: true })
  }
})
