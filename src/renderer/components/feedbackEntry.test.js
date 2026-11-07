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
