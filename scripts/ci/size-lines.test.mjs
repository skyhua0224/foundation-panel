import test from 'node:test'
import assert from 'node:assert/strict'
import { lineRanges } from './size-lines.mjs'

// a path under a crate's directory names the crate, anything else says nothing
const owner = (path) => {
  const crate = path.match(/\\(?:reg|ring)\\([\w.-]+)\\/)
  return crate ? 'crate:' + crate[1] : '[other source]'
}

const sample = [
  'Mod 0000 | `C:\\reg\\a\\a.o`:',
  'C:\\reg\\a\\a.rs (SHA-256: AB12)',
  '  0001:00000010-00000020, line/addr entries = 1',
  '    11 00000010 ! ',
  '  ',
  'C:\\reg\\b\\b.rs (MD5: 00FF)',
  '  0001:00000020-00000030, line/column/addr entries = 1',
  '    12 00000020 ! ',
  '  ',
  // not a known source path: it is charged to its object file, never to the file before it
  'C:\\other\\native.asm (None)',
  '  0001:00000030-00000040, line/addr entries = 1',
  '  0002:00000050-00000060, line/addr entries = 1',
  'Mod 0001 | `C:\\ring\\r\\x.o`:',
  // a range right after a module heading belongs to that object, not to the file before it
  '  0001:00000040-00000050, line/addr entries = 1',
  'Mod 0002 | `C:\\elsewhere\\y.o`:',
  '  0001:00000060-00000070, line/addr entries = 1',
].join('\n')

test('line ranges follow the file heading, with or without column information', () => {
  assert.deepEqual(lineRanges(sample, 1, owner), [
    { lo: 0x10, hi: 0x20, owner: 'crate:a' },
    { lo: 0x20, hi: 0x30, owner: 'crate:b' },
    { lo: 0x30, hi: 0x40, owner: 'crate:a' },
    { lo: 0x40, hi: 0x50, owner: 'crate:r' },
    { lo: 0x60, hi: 0x70, owner: '[other source]' },
  ])
})

test('only the requested section is returned', () => {
  assert.deepEqual(lineRanges(sample, 2, owner), [{ lo: 0x50, hi: 0x60, owner: 'crate:a' }])
})
