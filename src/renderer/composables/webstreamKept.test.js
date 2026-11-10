import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'

const src = fs.readFileSync(new URL('../components/WebStreamSettings.vue', import.meta.url), 'utf8')

test('the Moonlight Web page keeps the last status and offers no action before it is known', () => {
  assert.match(src, /<script>[^<]*let keptStatus = null[^<]*<\/script>/)
  assert.match(src, /\.\.\.\(keptStatus \|\| \{\}\)/)
  assert.match(src, /const known = ref\(!!keptStatus\)/)
  assert.match(src, /keptStatus = \{ \.\.\.status \}\s*\n\s*known\.value = true/)
  assert.match(src, /<template v-if="known" #actions>/)
  assert.match(src, /!known \? t\.webStream\.title :/)
})
