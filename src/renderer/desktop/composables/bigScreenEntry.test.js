import test from 'node:test'
import assert from 'node:assert/strict'
import { isBigScreenEntry, libraryApps } from './bigScreenEntry.js'

const entry = (name) => ({ name, detached: ['.\\assets\\gui\\sunshine-gui.exe --desktop'], 'prep-cmd': [{ do: '', undo: '.\\assets\\gui\\sunshine-gui.exe --close-desktop', elevated: 'false' }] })

test('the host entry is recognised by its commands in either language and under any name', () => {
  for (const name of ['Big Screen', '大屏', 'Couch']) assert.equal(isBigScreenEntry(entry(name)), true, name)
})

test('the Desktop app with the open-after-connect switch is not the entry: it has no close command', () => {
  assert.equal(isBigScreenEntry({ name: 'Desktop', detached: ['.\\assets\\gui\\sunshine-gui.exe --desktop'] }), false)
  assert.equal(isBigScreenEntry({ name: 'Desktop' }), false)
  assert.equal(isBigScreenEntry({ name: 'Steam Big Picture', cmd: 'steam://open/bigpicture' }), false)
  assert.equal(isBigScreenEntry(null), false)
})

test('the library lists everything but the entry and leaves a non-list alone', () => {
  const list = [{ name: 'Desktop' }, entry('Big Screen'), { name: 'Steam Big Picture' }]
  assert.deepEqual(libraryApps(list).map((a) => a.name), ['Desktop', 'Steam Big Picture'])
  assert.equal(libraryApps(undefined), undefined)
})
