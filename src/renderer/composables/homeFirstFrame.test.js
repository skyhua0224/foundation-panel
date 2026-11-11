import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'

// home-streaming-first-frame 2026-11-11 (owner, pk1111f): opening the overview, or switching to it, while a stream runs drew the idle overview first and flipped to the streaming one a moment later; the star card named no
// client until the clients arrived; the tiles of the stream grew when the second sample came. What the overview keeps between visits now includes the live state, the first reads go out together and the page
// is drawn once, when what decides it is known (ctx.pending). The behaviour is measured per frame (/var/tmp/ovf-qa/frames.mjs); these keep the pieces in place.
const home = readFileSync(new URL('../components/PanelHome.vue', import.meta.url), 'utf8')
const live = readFileSync(new URL('./usePanelHome.js', import.meta.url), 'utf8')

test('the overview keeps the session, the perf reading, the curves and the load between visits', () => {
  const keep = /const REMEMBER = \[([^\]]*)\]/.exec(live)?.[1] || ''
  for (const key of ['sessions', 'perf', 'perfAt']) assert.match(keep, new RegExp(`'${key}'`), `${key} is kept: a visit while a stream runs starts from it`)
  assert.match(live, /const SERIES_KEYS = \['bitrate', 'fps', 'latency', 'cpu', 'gpu', 'memory', 'loss', 'dropped'\]/)
  assert.match(live, /for \(const key of SERIES_KEYS\) state\.series\[key\] = \[\.\.\.remembered\.series\[key\]\]/)
  assert.match(live, /state\.sys\.cpu = remembered\.sys\.cpu/)
  assert.match(live, /push\(state\.series\.memory, memoryShare\(state\.memory\)\)\n    remember\(\)/, 'every poll of the session is kept')
})

test('the first reads go out together: the session is not asked after the log and the driver probe', () => {
  assert.match(live, /await Promise\.all\(\[facts\(\)\.then\(\(\) => hardware\(\)\), slow\(\), fast\(\)\]\)/)
  assert.doesNotMatch(live, /await Promise\.all\(\[facts\(\), slow\(\)\]\)\n\s+await Promise\.all\(\[fast\(\), hardware\(\)\]\)/)
  assert.match(live, /capTimer = setTimeout\(\(\) => \{ state\.settled = true \}, SETTLE_CAP_MS\)/, 'a host that answers slowly does not hold the page past the cap')
  assert.match(live, /state\.loaded = true\n    state\.settled = true/, 'kept data: settled in the first frame')
  assert.match(live, /clearTimeout\(capTimer\)/)
})

test('the overview says when it can be drawn (ctx.pending) and waits for the session, the clients, the DualSense keys of a stream and the covers; not for the system information', () => {
  assert.match(home, /const dashCtx = computed\(\(\) => \(\{ pending: pending\.value, live: !!session\.value,/)
  assert.match(home, /const pending = computed\(\(\) => !\(capped\.value \|\| \(state\.settled && \(!rawSession\.value \|\| startReady\.value\) && coverKnown\(catalogue\.value\['last-session'\]\?\.cover \|\| ''\)\)\)\)/)
  assert.match(home, /const startReady = computed\(\(\) => ds5Known\.value && coverKnown\(liveArt\.value\) && heroArtReady\(liveArt\.value\)\)/)
  assert.match(home, /setTimeout\(\(\) => \{ capped\.value = true \}, SETTLE_CAP_MS\)/)
  assert.doesNotMatch(/const pending = [^\n]*nics/.exec(home)?.[0] || '', /nics/, 'the system information only adds a tile on a bad link: it comes in with motion, it is not waited for')
})

test('the system information and the DualSense keys are kept between visits too', () => {
  assert.match(live, /export const seen = \{ nics: null, ds5Keys: null \}/)
  assert.match(home, /if \(seen\.nics\) nics\.info\.value = seen\.nics/); assert.match(home, /watch\(nics\.info, \(v\) => \{ if \(v\) seen\.nics = v \}\)/)
  assert.match(home, /const ds5Keys = ref\(seen\.ds5Keys \|\| \[\]\)/); assert.match(home, /seen\.ds5Keys = ds5Keys\.value; ds5Known\.value = true/)
})

test('a stream that starts while the page is open is one change: the session is held until its DualSense keys, cover and hero colours are known, and its perf reading comes with it', () => {
  assert.match(home, /const session = ref\(state\.settled \? rawSession\.value : null\)/)
  assert.match(home, /if \(!raw \|\| session\.value \|\| startReady\.value\) \{ session\.value = raw; return \}/, 'an end, and a stream already shown, follow the host at once')
  assert.match(home, /startTimer = setTimeout\(\(\) => \{ session\.value = rawSession\.value \}, SETTLE_CAP_MS\)/, 'a slow cover does not hold the stream back for good')
  assert.match(home, /watch\(\(\) => !!rawSession\.value, \(live\) => \{ if \(live\) loadDs5\(\) \}/)
  assert.match(live, /const perf = await safe\(\(\) => fetchJson\('\/api\/perf\/current'\)\)\n(?:.*\n)*?\s+state\.sessions = sessions\n\s+state\.perf = perf/, 'the session is assigned with its perf reading')
})
