<script setup>
import { inject, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { MPageLoading } from 'foundation-ui/vue'
import { invoke } from '@tauri-apps/api/core'
import { revealReport } from '../composables/frameReport.js'

// The embedded Web UI frame lives here for the whole session (so it never reloads when the window is resized). Where the
// list is hidden behind the page (single pane) a small bar carries the way back.
const scene = inject('m-scene', null)
// a page of the frame that is on a sub-page has its own back bubble: the way back to the list waits until it is on its first page
const props = defineProps({
  sub: { type: Boolean, default: false },
  /** the destination the frame shows ('' while the window shows a page of its own): a change means the frame is about to move to another page */
  page: { type: String, default: '' },
})

// While the frame moves to another page it is hidden and the content area shows the loading state (never the old page, never a blank or black frame). It is shown again when the
// frame has loaded (a `load` event after its address changed), or, when the page is told its place by a message and nothing reloads, after a short settle.
const host = ref(null)
const covered = ref(true)
// the first cover (a cold start) shows its loading state at once; the covers of page changes wait 300 ms (a transition delay), so a fast switch does not flash it
const first = ref(true)
// the skeleton (veil) is a timer of its own, not a CSS transition delay: where the system turns animations off (prefers-reduced-motion, a remote session) transitions, and with them their delay, are
// dropped, and the skeleton showed at once on every page change. It comes in after 300 ms of nothing painted, never before, and not at all on a page that paints sooner.
const veil = ref(true)
let veilTimer = 0
const VEIL_MS = 300
let waitLoad = false
let settle = 0
let ceiling = 0
let observer = null
// How long the frame stayed covered and what showed it is written to the panel's log when it was slow (>= 600 ms) or had to be forced by the ceiling: a blank right pane then has a time.
let coveredAt = 0
const uncover = (by = '', fcp = 0) => {
  clearTimeout(settle); clearTimeout(ceiling); waitLoad = false
  if (covered.value && coveredAt) {
    const line = revealReport({ page: props.page || 'frame', ms: performance.now() - coveredAt, by, fcp })
    if (line) invoke('frontend_slow', line).catch(() => {})
  }
  coveredAt = 0
  clearTimeout(veilTimer)
  covered.value = false
  veil.value = false
  first.value = false
}
const cover = () => {
  if (!covered.value || !coveredAt) coveredAt = performance.now()
  if (!covered.value) { veil.value = first.value; clearTimeout(veilTimer); if (!first.value) veilTimer = setTimeout(() => { veil.value = covered.value }, VEIL_MS) }
  covered.value = true; clearTimeout(ceiling); ceiling = setTimeout(() => uncover('ceiling'), 6000) }
// The frame is shown when its page says that it has painted its first frame (`panel-painted`, sent by the script the proxy puts into every page): the load event comes
// 50 to 100 ms before that, and a frame shown at load showed 4 to 6 frames with nothing in them. Where no such message comes (a page the proxy does not change) the load
// event shows the frame after a longer settle.
const onLoad = () => { if (!waitLoad && !covered.value) return; clearTimeout(settle); settle = setTimeout(() => uncover('load'), 450) }
const onMessage = (event) => {
  if (event.data?.type !== 'panel-painted' || !covered.value) return
  if (event.source !== host.value?.querySelector('iframe')?.contentWindow) return
  clearTimeout(settle)
  const fcp = event.data?.fcp
  requestAnimationFrame(() => uncover('painted', fcp))
}
onMounted(() => {
  host.value.addEventListener('load', onLoad, true)
  window.addEventListener('message', onMessage)
  // a frame whose address changes is a new element (SunshineFrame: a frame that navigates adds a history entry), so a frame that comes in counts like a changed `src`
  observer = new MutationObserver((records) => {
    if (!records.some((r) => r.type === 'attributes' || [...r.addedNodes].some((n) => n.tagName === 'IFRAME'))) return
    waitLoad = true; cover(); clearTimeout(settle)
  })
  observer.observe(host.value, { subtree: true, attributes: true, attributeFilter: ['src'], childList: true })
  cover()
})
// A page change inside the frame that is already open (the page is told its place, nothing reloads) is not covered at all: it is instant, and a skeleton there would only flash. A change that does
// move the frame (a new address) is covered by the observer above, and that cover shows only after 300 ms of nothing painted (CSS below).
watch(() => props.page, (page) => {
  if (!page) return
  if (!covered.value) return
  clearTimeout(settle)
  settle = setTimeout(() => { if (!waitLoad) uncover('settle') }, 320)
})
onBeforeUnmount(() => { host.value?.removeEventListener('load', onLoad, true); window.removeEventListener('message', onMessage); observer?.disconnect(); clearTimeout(settle); clearTimeout(ceiling) })
</script>

<template>
  <div ref="host" class="panel-web" :class="{ 'is-covered': covered, 'is-veil': covered && veil, 'is-first': first }">
    <div v-if="scene?.canBack.value && !sub" class="panel-web__bar">
      <button type="button" class="m-page__back" @click="scene.back()"><svg class="m-icon" viewBox="0 -960 960 960" aria-hidden="true"><path d="M313-440l224 224-57 56-320-320 320-320 57 56-224 224h487v80H313Z" /></svg></button>
    </div>
    <div class="panel-web__frame">
      <slot />
      <div class="panel-web__cover" aria-hidden="true"><MPageLoading :delay="0" /></div>
    </div>
  </div>
</template>

<style>
.panel-web { display: flex; flex-direction: column; height: 100%; min-height: 0; }
.panel-web__bar { padding: 8px 12px 0; flex: none; }
.panel-web__frame { position: relative; flex: 1; min-height: 0; }
.panel-web__frame .iframe-container, .panel-web__frame .sunshine-iframe { height: 100%; width: 100%; }
.panel-web__frame iframe { transition: opacity 140ms linear; }
/* not 0: a frame at opacity 0 is not painted for the paint timing, so its page paints only once shown (the 50 ms of empty frames) and never tells that it has painted; at 1 % it paints under the cover and nobody sees it */
.panel-web.is-covered .panel-web__frame iframe { opacity: 0.01; transition: none; }
.panel-web__cover { position: absolute; inset: 0; z-index: 1; overflow: hidden; padding: 76px var(--m-gutter, 32px) 0; box-sizing: border-box; pointer-events: none; opacity: 0; transition: opacity 120ms linear; }
.panel-web__cover > .m-pageload { width: min(100%, 960px); margin-inline: 0; }
@container (max-width: 700px) { .panel-web__cover { --m-gutter: 16px; } }
@container (max-width: 560px) { .panel-web__cover { --m-gutter: 12px; } }
.panel-web.is-veil .panel-web__cover { opacity: 1; }
/* the cover of a page change comes in after 300 ms of nothing painted; going away has no delay. The sweep of the rows runs on the mounted cover all the time (opacity does not restart it), so it does not begin again when .is-covered toggles */

@media (prefers-reduced-motion: reduce) { .panel-web__frame iframe, .panel-web__cover { transition: none; } }
</style>
