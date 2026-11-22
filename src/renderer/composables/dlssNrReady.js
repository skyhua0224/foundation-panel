import { computed, ref } from 'vue'

// The DLSS NR live controls (当前串流, 画质增强 › 当前会话) are shown only when the DLSS NR component is installed and usable: installed, with the
// host bridge and an NVIDIA adapter, the status EnhancementManager reads (native_component_get_status). Unknown (still reading, the call failed) hides
// them too, so they never flash in and out. The status is read once per panel and shared; the pages that read it anyway (画质增强, its manager) hand
// their answers in, so an install there shows the controls here without another call.

// A full component read hashes the runtime. Refresh it when the host reports a
// change that affects the install button, including the end of another stream's use.
export function nrComponentChanged(status, live) {
  return !status || status.installed !== live.nr_installed ||
    status.in_use !== live.component_in_use || status.maintenance !== live.component_maintenance
}

/** Whether a component status makes the live controls usable. */
export const dlssNrUsable = (status) => !!(status && status.installed && status.host_supported && status.adapter_present)

/**
 * What 画质增强 shows. Before DLSS NR is ready: the component state, the install flow, and the stream's output format (SDR / HDR / Dolby Vision on
 * 当前会话, the one thing of the NR overlay that is not NR). Only once it is ready: the overlay switch and its opacity, the overlay and NR shortcut keys
 * (shortcut 1 toggles NR, shortcut 2 shows the NR overlay window), the NR state of the stream, the live controls and the button that opens the overlay.
 */
export const qualityParts = (ready) => {
  const nr = ready === true
  return { components: true, install: true, session: true, output: true, overlay: nr, keys: nr, nrState: nr, liveControls: nr }
}

/** The shared answer: `state` is true / false, or null while unknown; `use()` reads once if nothing is known and gives `shown`. */
export function createDlssNrReady(getStatus) {
  const state = ref(null)
  let reading = null
  const note = (status) => { if (status && typeof status === 'object') state.value = dlssNrUsable(status) }
  const read = () => {
    reading ??= Promise.resolve().then(getStatus).then((result) => note(result?.success ? result.data : null), () => {}).finally(() => { reading = null })
    return reading
  }
  const shown = computed(() => state.value === true)
  const use = () => { if (state.value === null) void read(); return shown }
  return { state, shown, note, read, use }
}
