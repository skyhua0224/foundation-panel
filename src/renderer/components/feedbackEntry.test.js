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
  assert.equal((page.match(/<MFeedbackRow/g) || []).length, 1, 'one entry on the page, not two')
  const side = read('./SidebarMenu.vue')
  assert.match(side, /feedback: \(\) => \{ useErrors\(\)\.reportFeedback\(\) \}/)
  assert.match(side, /<MErrorHost :locale="locale"/, 'the form has a host in this window, so the row is always shown')
})

test('面板设置 has no account row any more: the report goes straight to the developers, with no GitHub sign-in', () => {
  const page = read('./ControlPanelPage.vue')
  assert.ok(!/MFeedbackAccount/.test(page))
})

test('the caption bar has the feedback icon next to the bell, and the context menu entry opens the one feedback panel with no text typed for the person', () => {
  const side = read('./SidebarMenu.vue')
  assert.match(side, /<MStatusBell :locale="locale" \/><MFeedbackButton :locale="locale" \/><MAnnouncementButton/)
  assert.match(side, /reportHere: \(\{ place, text \}\) => \{ void openFeedbackHere\(\{ place, text \}\) \},/)
  assert.ok(!/ctxReport/.test(side))
  assert.ok(!/reportHere[^\n]*developerMode/.test(side), 'the context menu callback does not ask for developer mode')
})

test('the tray menu opens the feedback panel, and a crash left by the previous run asks once whether to tell the developers', () => {
  const side = read('./SidebarMenu.vue')
  assert.match(side, /listen\('open-feedback', \(\) => \{ void openFeedbackHere\(\{\}\) \}\)/)
  assert.match(side, /if \(await invoke\('crash_recovered_ask'\)\.catch\(\(\) => false\)\) askDevReport\('crash'\)/)
  const rust = (p) => fs.readFileSync(new URL(`../../../src-tauri/src/${p}`, import.meta.url), 'utf8')
  assert.match(rust('tray/actions.rs'), /"feedback" => open_main_panel_and_emit\(app, "feedback", "open-feedback"/)
  assert.match(rust('tray/actions.rs'), /fn open_main_panel_and_emit[\s\S]*is_closing\("main"\)[\s\S]*after_closed\([\s\S]*emit_to_main_when_ready/, 'the event waits for a closing main window')
  assert.match(rust('tray/menu.rs'), /MenuItem::with_id\(app, "feedback", s\.feedback/)
  assert.match(rust('tray/popup_input.rs'), /"about", "feedback", "star_project"/)
  assert.match(rust('tray/popup_input.rs'), /put\("feedback", s\.feedback\)/)
  for (const word of ['反馈问题…', 'Send feedback…', 'フィードバックを送る…']) assert.ok(rust('tray.rs').includes(`feedback: "${word}"`), word)
  assert.match(rust('main.rs'), /commands::crash_recovered_ask,/)
  assert.match(rust('commands.rs'), /pub fn crash_recovered_ask\(\) -> bool \{\s*crate::crash::take_recovered\(\)/)
  assert.match(rust('crash/mod.rs'), /let \(ask, left\) = asked::unasked_at_start\(&pending\);\s*RECOVERED\.store\(ask, Ordering::SeqCst\);[\s\S]*spawn\(uploader\)/, 'the flag is set before the uploader can remove what is waiting, and only for a report not asked about yet')
  assert.match(rust('crash/mod.rs'), /pub\(crate\) fn take_recovered\(\) -> bool \{\s*RECOVERED\.swap\(false, Ordering::SeqCst\)\s*\}/, 'asking records nothing: a program that ends before the answer asks again')
  assert.match(rust('crash/mod.rs'), /pub\(crate\) fn mark_answered\(\)[\s\S]*asked::answered\(/, 'the answer is what is recorded: a report that stays pending (upload failed) does not bring the question back')
  assert.match(rust('main.rs'), /commands::crash_recovered_answered,/)
  assert.match(side, /watch\(\(\) => devReport\.decision, \(now, was\) => \{ if \(was === 'crash' && !now\) invoke\('crash_recovered_answered'\)/)
})
