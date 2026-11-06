import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'

// round 45: the big-screen dashboard has no filler line under its title, and the logs quick action carries the name of the page it opens (诊断与日志)
const read = (p) => readFileSync(new URL(p, import.meta.url), 'utf8')
const zh = read('../desktop/i18n/zh.js'); const en = read('../desktop/i18n/en.js'); const view = read('../desktop/views/DashboardView.vue')
const parts = read('./panelParts.js')

test('the dashboard title has no subtitle line', () => {
  assert.ok(!view.includes('hero-sub') && !view.includes('dashboard.subtitle'))
  assert.ok(!zh.includes('高性能游戏串流解决方案') && !en.includes('High-performance game streaming solution'))
})

test('the logs quick action is named 诊断与日志 / Diagnostics and logs and opens that page', () => {
  assert.ok(zh.includes("viewLogs: '诊断与日志',") && en.includes("viewLogs: 'Diagnostics and logs',"))
  assert.ok(!zh.includes('打开日志控制台') && !en.includes('Open Log Console'))
  assert.ok(view.includes("open_tool_window', { toolName: 'logs' }"))
  assert.ok(parts.includes("logs: { route: 'diagnostics'"), 'the tool name logs leads to the diagnostics route')
})
