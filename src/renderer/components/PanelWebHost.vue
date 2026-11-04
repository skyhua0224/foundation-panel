<script setup>
import { inject, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { MPageLoading } from 'foundation-ui/vue'

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
let waitLoad = false
let settle = 0
let ceiling = 0
let observer = null
const uncover = () => { clearTimeout(settle); clearTimeout(ceiling); waitLoad = false; covered.value = false }
const cover = () => { covered.value = true; clearTimeout(ceiling); ceiling = setTimeout(uncover, 6000) }
// The frame is shown when its page says that it has painted its first frame (`panel-painted`, sent by the script the proxy puts into every page): the load event comes
// 50 to 100 ms before that, and a frame shown at load showed 4 to 6 frames with nothing in them. Where no such message comes (a page the proxy does not change) the load
// event shows the frame after a longer settle.
const onLoad = () => { if (!waitLoad && !covered.value) return; clearTimeout(settle); settle = setTimeout(uncover, 450) }
const onMessage = (event) => {
  if (event.data?.type !== 'panel-painted' || !covered.value) return
  if (event.source !== host.value?.querySelector('iframe')?.contentWindow) return
  clearTimeout(settle)
  requestAnimationFrame(uncover)
}
onMounted(() => {
  host.value.addEventListener('load', onLoad, true)
  window.addEventListener('message', onMessage)
  observer = new MutationObserver(() => { waitLoad = true; cover(); clearTimeout(settle) })
  observer.observe(host.value, { subtree: true, attributes: true, attributeFilter: ['src'] })
  cover()
})
watch(() => props.page, (page) => {
  if (!page) return
  cover()
  clearTimeout(settle)
  settle = setTimeout(() => { if (!waitLoad) uncover() }, 320)
})
onBeforeUnmount(() => { host.value?.removeEventListener('load', onLoad, true); window.removeEventListener('message', onMessage); observer?.disconnect(); clearTimeout(settle); clearTimeout(ceiling) })
</script>

<template>
  <div ref="host" class="panel-web" :class="{ 'is-covered': covered }">
    <div v-if="scene?.canBack.value && !sub" class="panel-web__bar">
      <button type="button" class="m-page__back" @click="scene.back()"><svg class="m-icon" viewBox="0 -960 960 960" aria-hidden="true"><path d="M313-440l224 224-57 56-320-320 320-320 57 56-224 224h487v80H313Z" /></svg></button>
    </div>
    <div class="panel-web__frame">
      <slot />
      <div class="panel-web__cover" aria-hidden="true"><MPageLoading :delay="140" /></div>
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
.panel-web__cover { position: absolute; inset: 0; z-index: 1; overflow: hidden; padding: 76px 16px 0; box-sizing: border-box; pointer-events: none; opacity: 0; transition: opacity 120ms linear; }
.panel-web__cover > .m-pageload { width: min(100%, 840px); margin-inline: auto; }
.panel-web.is-covered .panel-web__cover { opacity: 1; }
@media (prefers-reduced-motion: reduce) { .panel-web__frame iframe, .panel-web__cover { transition: none; } }
</style>
