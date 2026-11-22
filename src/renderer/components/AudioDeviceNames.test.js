import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { audioDeviceName } from 'foundation-ui/settings/index.js'

// The audio device of the host is stored as an endpoint id ({0.0.0.00000000}.{…}). The settings list and the overview tile print what the
// window gives them, so the window gives them the playback devices of the host (the same answer the microphone page reads) and the row shows a name.
const sidebar = readFileSync(new URL('./SidebarMenu.vue', import.meta.url), 'utf8')
const home = readFileSync(new URL('./PanelHome.vue', import.meta.url), 'utf8')

test('the settings list of the sidebar reads the playback devices of the host', () => {
  assert.match(sidebar, /import \{ audioDevicesChannel \} from '\.\.\/composables\/deviceStatusStore\.js'/)
  assert.match(sidebar, /useSettingsList\(\{.*usb: computed\(\(\) => runtime\.usb\), audio: computed\(\(\) => audioDevicesChannel\.data\.value\?\.render \|\| null\)/)
  assert.match(sidebar, /onMounted\(\(\) => \{ audioDevicesChannel\.fetch\(\)/)
})

test('the overview gives its tiles the playback devices of the host', () => {
  assert.match(home, /import \{ audioDevicesChannel \} from '\.\.\/composables\/deviceStatusStore\.js'/)
  assert.match(home, /config: state\.config, audio: audioDevicesChannel\.data\.value\?\.render \|\| null,/)
})

test('a chosen device shows by its name, and by words when the list is not there, never as the id', () => {
  const id = '{0.0.0.00000000}.{172f61d5-5aff-4b19-9233-e02c0003996f}'
  const render = [{ id, name: 'CABLE Input (VB-Audio Virtual Cable)', description: 'CABLE Input', adapter: 'VB-Audio Virtual Cable' }]
  assert.equal(audioDeviceName(id, render, '所选设备'), 'CABLE Input (VB-Audio Virtual Cable)')
  assert.equal(audioDeviceName(id, null, '所选设备'), '所选设备')
})
