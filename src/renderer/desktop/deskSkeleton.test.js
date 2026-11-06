import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import test from 'node:test'

const root = fileURLToPath(new URL('./', import.meta.url))
const read = (p) => readFileSync(join(root, p), 'utf8')

test('each of the six async views of the big screen has a skeleton of its kind, shown from the first frame', () => {
  const app = read('DesktopApp.vue')
  for (const kind of ['apps', 'dashboard', 'devices', 'stream', 'tools', 'settings']) {
    assert.match(app, new RegExp(`viewLoader\\('${kind}', \\(\\) => import\\('./views/`), kind)
  }
  assert.match(app, /delay: 0/, 'no delay: the first frames of a cold view are not empty')
  assert.doesNotMatch(app, /= defineAsyncComponent\(\(\) =>/, 'no async view without a loading state')
})

test('the skeleton has nothing to focus, so a pad keeps its place while the view loads', () => {
  const sk = read('components/DeskViewSkeleton.vue')
  const template = sk.slice(sk.indexOf('<template>'), sk.indexOf('<style>'))
  assert.doesNotMatch(template, /tabindex|data-focusable|data-focus-key|<button|<a /)
  assert.match(sk, /prefers-reduced-motion/)
  assert.match(sk, /data-motion='reduced'/)
  assert.doesNotMatch(sk, /#[0-9a-fA-F]{3,8}\b/, 'colours are tokens')
})

test('the big-screen dashboard draws the memory card as bars until the first reading, not 「no process」', () => {
  const dash = read('views/DashboardView.vue')
  assert.match(dash, /<div v-if="!memoryLoaded" class="mem-skel"/)
  assert.match(dash, /class="no-process" v-else-if="memoryLoaded"/)
  assert.match(dash, /\} finally \{\n    memoryLoaded\.value = true/)
})
