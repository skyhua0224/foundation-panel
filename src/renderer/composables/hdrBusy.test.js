import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'

// 「HDR operation rejected: HTTP 409 / hdr_save_busy」 (the log of the installed build, every few minutes): the host holds a transaction while it stores or applies something and refuses the
// next read or write. It was logged as a warning and told to the page as the wrong error ("component is in use"). Now the panel asks again, the page says 「正在保存…」, and nothing is dropped.
const read = (p) => fs.readFileSync(new URL(p, import.meta.url), 'utf8')
const rs = read('../../../src-tauri/src/hdr_enhanced.rs')

test('a busy host is asked again with a backoff, reads and writes, before any error is returned', () => {
  assert.match(rs, /let delays: &\[u64\] = if writes \{ &\[200, 400, 800, 1600, 2400, 2400, 2400\] \} else \{ &\[150, 300, 600, 900, 1100\] \};/)
  assert.match(rs, /Err\(error\) if error == "hdr_save_busy" && attempt < delays\.len\(\)/)
  assert.ok(rs.indexOf('(409, Some("hdr_save_busy"))') > 0 && rs.indexOf('(409, Some("hdr_save_busy"))') < rs.indexOf('(409, _) => "HDR-OP-001'), 'its own code before the generic 409')
  assert.match(rs, /if code != "nr_settings_pending" && code != "hdr_save_busy" \{\s*log::warn!/, 'a busy answer is not a warning')
})

test('the page says 「正在保存…」 and tries once more; it does not drop the save', () => {
  const manager = read('./useEnhancementManager.js')
  assert.match(manager, /includes\('hdr_save_busy'\)\) \{\s*ElMessage\.info\(text\.value\.saveBusy \|\| '正在保存…'\)\s*await new Promise\(\(resolve\) => setTimeout\(resolve, 1500\)\)\s*result = await api\.setEnabled\(enabled\)/)
  const controls = read('./enhancementControls.js')
  assert.match(controls, /hdr_save_busy: '正在保存…'/); assert.match(controls, /hdr_save_busy: 'Saving…'/)
})
