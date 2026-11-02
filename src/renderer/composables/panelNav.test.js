import assert from 'node:assert/strict'
import fs from 'node:fs'
import test from 'node:test'
import { zh } from '../desktop/i18n/zh.js'
import { en } from '../desktop/i18n/en.js'

// the router and the nav need the shared package (the bundler resolves it), so the destinations are read from the source
const router = fs.readFileSync(new URL('./useRouter.js', import.meta.url), 'utf8')
const nav = fs.readFileSync(new URL('./panelNav.js', import.meta.url), 'utf8')

test('米塔 AI is a destination of the rail, the chat takes the whole width, and it has left the 系统 list', () => {
  assert.match(router, /export const RAILS = \[[^\]]*'system', 'assistant', 'settings'/)
  assert.match(router, /\[ROUTES\.AI_ASSISTANT\]: \{ rail: 'assistant', list: false \}/)
  const system = nav.slice(nav.indexOf('system: ['), nav.indexOf('],', nav.indexOf('system: [')))
  assert.ok(!/AI_ASSISTANT/.test(system), 'not in the 系统 list')
  assert.match(nav, /assistant: 'smart_toy'/, 'a monochrome Material Symbol like every other destination')
  assert.ok(!/mita-rail|image:/.test(nav), 'no colour picture in the rail')
})
test('米塔 AI is labelled 米塔 AI (Mita AI), never AI 助手', () => {
  for (const [t, label, banned] of [[zh, '米塔 AI', /AI 助手/], [en, 'Mita AI', /AI Assistant/i]]) {
    assert.equal(t.m3e.nav.assistant, label)
    assert.equal(t.aiAssistant.title, label)
    assert.equal(t.sidebar.aiAssistant, label)
    assert.ok(!banned.test(t.aiAssistant.title))
  }
})
