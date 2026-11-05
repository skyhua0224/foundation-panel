import { test } from 'node:test'
import assert from 'node:assert/strict'
import { capturedDisplay, pickStreamMonitor, centredOn } from './streamMonitor.js'

const devices = [
  { device_id: '{a1}', data: 'DISPLAY NAME: \\\\.\\DISPLAY1\nFRIENDLY NAME: DELL U2723QE' },
  { device_id: '{b2}', data: 'DISPLAY NAME: \\\\.\\DISPLAY12\nFRIENDLY NAME: Virtual Display' },
]
const mon = (name, x, scale = 1) => ({ name, position: { x, y: 0 }, size: { width: 2560 * scale, height: 1440 * scale }, scaleFactor: scale })
const monitors = [mon('\\\\.\\DISPLAY1', 0, 1.5), mon('\\\\.\\DISPLAY12', 3840)]

test('the captured display is the DISPLAYn of the device Sunshine names', () => {
  assert.equal(capturedDisplay('{b2}', devices), 'DISPLAY12')
  assert.equal(capturedDisplay('{a1}', devices), 'DISPLAY1')
  assert.equal(capturedDisplay('\\\\.\\DISPLAY3', []), 'DISPLAY3')
  assert.equal(capturedDisplay('', devices), '')
})

test('the stream monitor is that display (DISPLAY1 is not DISPLAY12); auto is the primary; a display that is gone falls back to the primary, not found', () => {
  assert.equal(pickStreamMonitor({ outputName: '{b2}', displayDevices: devices, monitors, primary: monitors[0] }).monitor, monitors[1])
  assert.equal(pickStreamMonitor({ outputName: '{a1}', displayDevices: devices, monitors, primary: monitors[1] }).monitor, monitors[0])
  assert.deepEqual(pickStreamMonitor({ outputName: '', displayDevices: devices, monitors, primary: monitors[0] }), { monitor: monitors[0], found: true })
  assert.deepEqual(pickStreamMonitor({ outputName: '{zz}', displayDevices: devices, monitors, primary: monitors[0] }), { monitor: monitors[0], found: false })
  assert.deepEqual(pickStreamMonitor({ outputName: '', monitors: [] }), { monitor: null, found: false })
})

test('a name the host wrote itself (ZakoHDR) is the device whose friendly name is Zako HDR', () => {
  const zako = [{ device_id: '{9d2d}', data: 'DISPLAY NAME: \\\\.\\DISPLAY15\nFRIENDLY NAME: Zako HDR\nDEVICE STATE: PRIMARY\nHDR STATE: ENABLED' }]
  assert.equal(capturedDisplay('ZakoHDR', zako), 'DISPLAY15')
  assert.equal(capturedDisplay('zako hdr', zako), 'DISPLAY15')
  assert.equal(capturedDisplay('Other', zako), '')
  const m = [mon('\\\\.\\DISPLAY15', 0, 2)]
  assert.deepEqual(pickStreamMonitor({ outputName: 'ZakoHDR', displayDevices: zako, monitors: m, primary: m[0] }), { monitor: m[0], found: true })
})

test('the window is centred on the monitor in logical pixels', () => {
  assert.deepEqual(centredOn(monitors[0], 960, 600), { x: 800, y: 420, width: 960, height: 600 })
  assert.deepEqual(centredOn(monitors[1], 960, 600), { x: 3840 + 800, y: 420, width: 960, height: 600 })
})
