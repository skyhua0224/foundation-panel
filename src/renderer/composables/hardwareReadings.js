// HWiNFO labels that carry the figures of the tiles (its shared memory is the only source of CPU and GPU load here).
const HW = {
  cpuLoad: /^total cpu usage$/i,
  gpuLoad: /^gpu core load$|^gpu utilization$/i,
  cpuTemp: /^cpu package$|^cpu \(tctl\/tdie\)$|^cpu die \(average\)$/i,
  gpuTemp: /^gpu temperature$/i,
}
/** Pick the CPU and GPU figures out of HWiNFO readings; fields stay null when the label is not there. */
export function pickHardware(readings) {
  const find = (re, type) => (readings || []).find((r) => (!type || r.reading_type === type) && re.test(String(r.label_original || r.label_user || '')))
  const num = (r) => (r && Number.isFinite(r.value) ? r.value : null)
  return {
    cpu: num(find(HW.cpuLoad, 'Usage')), gpu: num(find(HW.gpuLoad, 'Usage')),
    cpuTemp: num(find(HW.cpuTemp, 'Temp')), gpuTemp: num(find(HW.gpuTemp, 'Temp')),
  }
}

