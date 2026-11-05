import assert from 'node:assert/strict'
import fs from 'node:fs'
import test from 'node:test'

// The standalone offline page the proxy serves keeps the original copy of the old page word for word (the same strings are in the package's offlineTexts).
const page = fs.readFileSync(new URL('../../../src-tauri/error-404.html', import.meta.url), 'utf8')

test('the standalone error page keeps the original copy and its retry', () => {
  for (const text of ['呜～Sunshine 好像被玩坏掉了呢～', '杂鱼～连 Sunshine 都启动不了，真是没用呢♡', '让 Sunshine 重新站起来的方法：', 'Sunshine 服务是不是在偷懒睡觉呢～', '端口 47990 准备好了吗？杂鱼要好好确认哦', '防火墙可不能把 Sunshine 关在外面呢～', '再试一次', '快点让 Sunshine 醒过来嘛～', '哎呀！Sunshine好像睡着了～', '醒过来啦！正在跳转...', '次探测失败，']) assert.ok(page.includes(text), text)
  assert.match(page, /prefers-color-scheme: dark/)
  assert.match(page, /PHASE1_DURATION = 10/)
})
