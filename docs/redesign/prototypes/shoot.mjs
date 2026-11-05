// Renders every prototype page in a real browser: node shoot.mjs <protoOut> <evidenceRoot> [variants]
import fs from 'node:fs'
import path from 'node:path'
import { createRequire } from 'node:module'
const require = createRequire('/home/skyhua/node_modules/')
const { chromium } = require('playwright-core')

const [, , protoOut, evidence, only] = process.argv
const names = { a: 'a-cards', b: 'b-list', c: 'c-rail' }
const pages = [['overview', '概览'], ['gamepad', '手柄'], ['microphone', '麦克风']]
const sizes = { normal: { width: 1440, height: 900 }, narrow: { width: 760, height: 900 }, min: { width: 1024, height: 640 } }
const browser = await chromium.launch({ executablePath: '/home/skyhua/.cache/ms-playwright/chromium-1237/chrome-linux64/chrome', args: ['--no-sandbox', '--disable-gpu', '--font-render-hinting=none'] })
const problems = []
for (const key of (only || 'a,b,c').split(',')) {
  const dir = path.join(evidence, names[key])
  fs.mkdirSync(dir, { recursive: true })
  const cells = []
  for (const [size, viewport] of Object.entries(sizes)) {
    for (const theme of ['light', 'dark']) {
      const ctx = await browser.newContext({ viewport, deviceScaleFactor: 1 })
      const page = await ctx.newPage()
      for (const [id, label] of pages) {
        await page.goto(`file://${path.join(protoOut, key, `${id}-${theme}.html`)}`)
        await page.evaluate(() => document.fonts.ready)
        // Overflow check: nothing may force a horizontal scrollbar on the page or its scroll container.
        const overflow = await page.evaluate(() => {
          const sc = document.querySelector('.main, .content, .detail') || document.documentElement
          const bad = []
          for (const el of document.querySelectorAll('.card, .group, .opt, .tile, .seg, .row, .fact, .preset, .tab, .status-chips, .stat')) {
            const r = el.getBoundingClientRect()
            if (r.right > window.innerWidth + 0.5) bad.push((el.className || el.tagName) + ':' + Math.round(r.right))
          }
          return { scrollW: document.documentElement.scrollWidth, innerW: window.innerWidth, bad: bad.slice(0, 5) }
        })
        if (overflow.scrollW > overflow.innerW || overflow.bad.length) problems.push({ key, id, theme, size, ...overflow })
        const file = path.join(dir, `${id}-${theme}-${size}.png`)
        await page.screenshot({ path: file })
        cells.push({ id, label, theme, size, file })
        // Full-length capture of the long page: grow the window until the content area no longer scrolls.
        if (id === 'gamepad' && ((size === 'normal' && theme === 'light') || (size === 'narrow' && theme === 'dark'))) {
          const extra = await page.evaluate(() => { const m = document.querySelector('.main'); return m.scrollHeight - m.clientHeight })
          await page.setViewportSize({ width: viewport.width, height: viewport.height + extra })
          await page.screenshot({ path: path.join(dir, `${id}-${theme}-${size}-full.png`) })
          await page.setViewportSize(viewport)
        }
      }
      await ctx.close()
    }
  }
  // Contact sheet: rows = pages, columns = light/dark at normal width, then light/dark narrow.
  const order = [['normal', 'light'], ['normal', 'dark'], ['narrow', 'light'], ['narrow', 'dark']]
  const title = { a: 'A · 卡片 Cards', b: 'B · 列表 Settings list', c: 'C · 导轨 Rail + status bar' }[key]
  const html = `<!doctype html><meta charset="utf-8"><style>
    body{margin:0;padding:20px;background:#fff;font:14px 'Noto Sans CJK SC','Noto Sans SC',sans-serif;color:#222}
    h1{font-size:20px;margin:0 0 14px} .row{display:flex;gap:14px;margin-bottom:18px;align-items:flex-start}
    figure{margin:0} figcaption{font-size:12px;color:#555;margin:4px 0}
    img{display:block;border:1px solid #ccd;height:300px;width:auto}
  </style><h1>${title}</h1>${pages.map(([id, label]) => `<div class="row">${order.map(([size, theme]) => {
    const c = cells.find((x) => x.id === id && x.size === size && x.theme === theme)
    return `<figure><figcaption>${label} · ${theme === 'light' ? '浅色' : '深色'} · ${size === 'normal' ? '1440' : '760'} px</figcaption><img src="file://${c.file}"></figure>`
  }).join('')}</div>`).join('')}`
  const sheetHtml = path.join(dir, '.sheet.html')
  fs.writeFileSync(sheetHtml, html)
  const sheet = await browser.newPage({ viewport: { width: 1640, height: 900 } })
  await sheet.goto('file://' + sheetHtml)
  await sheet.waitForLoadState('load')
  await sheet.screenshot({ path: path.join(dir, 'contact-sheet.png'), fullPage: true })
  fs.rmSync(sheetHtml)
}
await browser.close()
console.log(JSON.stringify({ problems }, null, 2))
