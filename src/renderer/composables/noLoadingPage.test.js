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
  assert.ok(!/proxyHealthFailed|loading-overlay/.test(frame), "the frame draws no state of its own: a proxy that does not answer is one cause of the shell's status page")
  const shell = read("../components/SidebarMenu.vue")
  assert.ok(/offlineCause = computed\(\(\) => \((?:hostSimulated\.value === 'offline' \? 'down' : )?(?:hostSimulated\.value === 'unresponsive' \? 'unresponsive' : )?proxyBlock\.blocked \? 'proxy'/.test(shell), "the proxy is one cause of the one status page")
})

test("the start of the panel shows no splash text", () => {
  const html = read("../sunshine-frame.html")
  assert.ok(!/boot-loading|Preparing Sunshine/.test(html))
  assert.ok(/<div id="app"><\/div>/.test(html))
  assert.ok(!/正在准备 Sunshine/.test(read("../sunshine-frame.js")))
})
