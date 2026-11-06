import assert from 'node:assert/strict'
import fs from 'node:fs'
import test from 'node:test'

// The standalone offline page the proxy serves says what the package's offlineTexts say (service, port, firewall, the way back) in the same plain words; the old wording is gone.
const page = fs.readFileSync(new URL('../../../src-tauri/error-404.html', import.meta.url), 'utf8')

test('the standalone error page tells what to check and retries by itself', () => {
  for (const text of ['Sunshine 没有响应', 'Sunshine 没有在运行，或者没有回应请求。', '可以这样排查：', 'Sunshine 服务有没有在运行', '端口 47990 有没有被占用或被拦截', '防火墙有没有拦截 Sunshine', '再试一次', 'Sunshine 恢复后，这个页面会自动返回。', 'Sunshine 已恢复，正在返回…', '次探测失败', '正在探测…', 'tip-ic']) assert.ok(page.includes(text), text)
  for (const old of ['杂鱼', '♡', '呜～', '偷懒', '醒过来', '玩坏', '睡着', '🎀', '🎉', '⏰', '🔍']) assert.ok(!page.includes(old), old)
  assert.match(page, /prefers-color-scheme: dark/)
  assert.match(page, /PHASE1_DURATION = 10/)
  // the retry gives feedback (busy button, turning ring) and each tip has its icon, like the in-app state
  assert.equal((page.match(/class="tip-ic"/g) || []).length, 3)
  assert.match(page, /id="retry"/)
  assert.match(page, /classList\.add\('busy'\)/)
})
