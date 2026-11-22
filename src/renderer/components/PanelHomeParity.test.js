import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'

const home = fs.readFileSync(new URL('./PanelHome.vue', import.meta.url), 'utf8')
const live = fs.readFileSync(new URL('../composables/liveSession.js', import.meta.url), 'utf8')

// the overview of the control panel and of the Web UI are one planner over one catalogue: what differs is only the data each product hands in
test("the host audio tile's switch sends the command of the stream; a refusal puts the switch back", () => {
  assert.match(home, /case 'runtime-host-audio': setHostAudio\(a\.value\); break/)
  assert.match(home, /hostApi\.postJson\('\/api\/runtime\/audio', \{ clientname: client, enabled \}\)/)
  assert.match(home, /catch \(e\) \{ failed\?\.\(\); say\(/)
  // one command per client at a time, in the order chosen: the last choice wins
  assert.match(home, /const setHostAudio = createQueuedCalls\(async \(\{ client, enabled, failed \}\) => \{/)
  assert.match(home, /\}, \(a\) => a\.client\)\n/)
})

test('the stages and the extras of a perf session are foundation-ui\'s, the same functions as the Web UI', () => {
  assert.match(live, /export \{ PIPELINE_FIELDS, pipelineAverages, perfExtras \} from 'foundation-ui\/home\/perf'/)
  assert.ok(!/export function pipelineAverages/.test(live), 'not a second copy')
})

test('the smart plan reads the startup problems of the whole host log, as the Web UI does (the list shown is cut to the latest 30)', () => {
  assert.match(home, /problems: startupProblems\(\(parseErrorLog\(state\.logText, Infinity\)\?\.rows \|\| \[\]\)\.filter\(\(r\) => r\.level === 'Fatal'\)\.map\(\(r\) => r\.text\), String\(locale\.value\)\)\.length/)
  assert.match(home, /startupProblems, parseErrorLog,/)
})
