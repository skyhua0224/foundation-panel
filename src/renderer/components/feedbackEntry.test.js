import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'

// 反馈: a row in 帮助与关于 and an action of the search, both opening the form of MErrorHost (mounted for the whole window)
const read = (p) => fs.readFileSync(new URL(p, import.meta.url), 'utf8')

test('帮助与关于 has the feedback row next to 开始体检, and the search can run 反馈', () => {
  const page = read('./ControlPanelPage.vue')
  assert.match(page, /MFeedbackRow :locale="locale"/)
  assert.ok(page.indexOf('doctorTexts(locale).start') < page.indexOf('<MFeedbackRow'), 'the row follows the health check row')
  assert.match(page, /import \{[^}]*MFeedbackRow[^}]*\} from 'foundation-ui\/vue'/)
  const side = read('./SidebarMenu.vue')
  assert.match(side, /feedback: \(\) => \{ useErrors\(\)\.reportFeedback\(\) \}/)
  assert.match(side, /<MErrorHost :locale="locale"/, 'the form has a host in this window, so the row is always shown')
})

test('面板设置 has 账号与反馈 for every user: the GitHub account and the feedback row, not behind developer mode', () => {
  const page = read('./ControlPanelPage.vue')
  assert.match(page, /<MFeedbackAccount :locale="locale" \/>/)
  assert.match(page, /import \{[^}]*MFeedbackAccount[^}]*\} from 'foundation-ui\/vue'/)
  assert.ok(page.indexOf('<MFeedbackAccount') > page.indexOf('t.sidebar.accountSettings') && page.indexOf('<MFeedbackAccount') < page.indexOf('t.sidebar.maintenance'), 'it sits with the account settings')
  assert.ok(!/<MFeedbackAccount[^>]*v-if/.test(page), 'no condition on the section')
  const side = read('./SidebarMenu.vue')
  assert.match(side, /reportHere: \(/)
  assert.ok(!/reportHere[^\n]*developerMode/.test(side), 'the context menu callback does not ask for developer mode')
})
