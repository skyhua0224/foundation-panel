import test from 'node:test'
import assert from 'node:assert/strict'
import { SPONSORS as ABOUT } from 'foundation-ui/about'
import { SPONSORS, sponsorUrl } from './sponsors.js'

test('sponsor targets are the team\'s own 爱发电 pages in team order, the same list as the about page', () => {
  assert.deepEqual(SPONSORS.map((s) => [s.label, s.url]), ABOUT.map((s) => [s.login, s.url]))
  assert.deepEqual(SPONSORS.map((s) => s.label), ['qiin2333', 'Yundi339', 'skyhua0224', '微信赞赏 / WeChat', 'GitHub Sponsors'])
  for (const s of SPONSORS) assert.match(s.url, /^https:\/\//)
})

// The tray's support line (m3e SUPPORT_ROW; src-tauri/src/tray/actions.rs): the sponsor chooser runs sponsor_<id>, and the host opens the same page the renderer's list names.
test('the tray menu opens the sponsor pages of this list, in the same order, and the website, channel and star addresses', async () => {
  const { readFileSync } = await import('node:fs')
  const { SUPPORT_ROW, ITEMS } = await import('foundation-ui/tray/trayModel.js')
  const rust = readFileSync(new URL('../../../src-tauri/src/tray/actions.rs', import.meta.url), 'utf8')
  assert.deepEqual(SUPPORT_ROW.sponsors, SPONSORS.map((s) => `sponsor_${s.id}`))
  assert.deepEqual(SUPPORT_ROW.sponsors.map((id) => ITEMS[id].label), SPONSORS.map((s) => s.label))
  for (const s of SPONSORS.filter((entry) => entry.id !== 'wechat')) assert.match(rust, new RegExp(`"sponsor_${s.id}" => utils::open_url_in_browser\\("${s.url.replace(/[.\/]/g, '\\$&')}"\\)`), s.id)
  assert.match(rust, /BILIBILI_URL: &str = "https:\/\/space\.bilibili\.com\/3690974838524514"/)
  assert.match(rust, /STAR_URL: &str = "https:\/\/github\.com\/AlkaidLab\/foundation-sunshine"/)
  assert.match(rust, /"https:\/\/www\.alkaidlab\.cn\/"[\s\S]*"https:\/\/www\.alkaidlab\.com\/"/)
  assert.match(rust, /"sponsor_wechat" => utils::open_url_in_browser\(sponsor_site_url\(&get_current_locale\(\)\)\)/)
  const menu = readFileSync(new URL('../../../src-tauri/src/tray/menu.rs', import.meta.url), 'utf8')
  for (const id of [...SUPPORT_ROW.chips, ...SUPPORT_ROW.sponsors]) assert.ok(menu.includes(`"${id}"`), `the native menu carries ${id}`)
})

test('wechat sponsor follows the current language', () => {
  const wechat = SPONSORS.find((s) => s.id === 'wechat')
  assert.equal(sponsorUrl(wechat, 'zh-CN'), 'https://www.alkaidlab.cn/#sponsors')
  assert.equal(sponsorUrl(wechat, 'zh-TW'), 'https://www.alkaidlab.cn/#sponsors')
  assert.equal(sponsorUrl(wechat, 'en'), 'https://www.alkaidlab.com/en/#sponsors')
})
