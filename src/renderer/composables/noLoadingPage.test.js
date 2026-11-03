import test from "node:test"
import assert from "node:assert/strict"
import { readFileSync } from "node:fs"

const read = (path) => readFileSync(new URL(path, import.meta.url), "utf8")

// There is no loading page: a page switch shows the new shell at once (the page fills in with its own skeletons), and the start shows the window background, not a splash.
test("the frame host has no full-area loading screen and never hides the page while it loads", () => {
  const frame = read("../components/SunshineFrame.vue")
  assert.ok(!/v-show="!loading"/.test(frame), "the iframe is not hidden while loading")
  assert.ok(!/vivi-loading/.test(frame))
  assert.ok(!/t\.sunshineFrame\.preparing/.test(frame))
  assert.ok(/v-if="proxyHealthFailed"/.test(frame), "only a proxy that does not answer is shown over the page")
})

test("the start of the panel shows no splash text", () => {
  const html = read("../sunshine-frame.html")
  assert.ok(!/boot-loading|Preparing Sunshine/.test(html))
  assert.ok(/<div id="app"><\/div>/.test(html))
  assert.ok(!/正在准备 Sunshine/.test(read("../sunshine-frame.js")))
})
