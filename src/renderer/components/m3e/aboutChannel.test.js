import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'

const read = (name) => readFileSync(new URL(name, import.meta.url), 'utf8')

test('关于 shows 更新通道 from the panel\'s preference and changes it with the handler of 面板设置', () => {
  const page = read('./AboutPage.vue')
  assert.match(page, /:update-channel="includePrerelease \? 'prerelease' : 'stable'"/)
  assert.match(page, /@channel-change="\(value\) => emit\('channel-change', value === 'prerelease'\)"/)
  const menu = read('../SidebarMenu.vue')
  const about = menu.split('\n').find((line) => line.includes('<AboutPage '))
  assert.match(about, /:include-prerelease="includePrerelease"/)
  assert.match(about, /@channel-change="setIncludePrerelease"/)
  // the same handler as the switch of 面板设置, so the two always show the same channel
  assert.match(menu, /<ControlPanelPage[\s\S]*?:include-prerelease="includePrerelease"[\s\S]*?@channel-change="setIncludePrerelease"/)
})
