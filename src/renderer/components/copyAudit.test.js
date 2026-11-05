import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

// The copy audit: a supporting line must carry information its title does not. These lines only restated their title or
// talked about the interface itself, so they were removed; they must not come back.
const here = dirname(fileURLToPath(import.meta.url))
const read = (path) => readFileSync(join(here, path), 'utf8')
const TEXT_FILES = ['../desktop/i18n/zh.js', '../desktop/i18n/en.js', 'ControlPanelPage.vue', 'VddSettings.vue']

const BANNED = [
  '查看串流剪贴板同步的连接状态', 'View the connection status of streaming clipboard sync',
  '修改 WebUI 用户名与密码', 'Change the Web UI username and password',
  '重新配置基础设置', 'Configure the basics again',
  '到哪里查找和下载新版本', '你正在主机本机上',
]

test('lines that only restated their title are gone from the panel', () => {
  for (const file of TEXT_FILES) {
    const text = read(file)
    for (const phrase of BANNED) assert.ok(!text.includes(phrase), `${file}: ${phrase}`)
  }
})

test('the help page rows read no line that the shared texts no longer have', () => {
  const page = read('ControlPanelPage.vue')
  assert.ok(!page.includes('coach.rerunText') && !page.includes('remote.lead'))
  assert.ok(!read('VddSettings.vue').includes('vp.advancedHint'))
})
