import test from 'node:test'
import assert from 'node:assert/strict'
import { pickHardware } from './hardwareReadings.js'

test('CPU and GPU figures are picked by their HWiNFO labels and left empty when absent', () => {
  const readings = [
    { reading_type: 'Usage', label_original: 'Total CPU Usage', value: 18.5 },
    { reading_type: 'Usage', label_original: 'GPU Core Load', value: 61 },
    { reading_type: 'Temp', label_original: 'CPU Package', value: 54 },
    { reading_type: 'Usage', label_original: 'Core 0 Usage', value: 99 },
  ]
  assert.deepEqual(pickHardware(readings), { cpu: 18.5, gpu: 61, cpuTemp: 54, gpuTemp: null })
  assert.deepEqual(pickHardware(undefined), { cpu: null, gpu: null, cpuTemp: null, gpuTemp: null })
})
