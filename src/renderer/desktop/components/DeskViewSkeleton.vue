<script setup>
import { computed } from 'vue'
import { MHero } from 'foundation-ui/vue'
import { useDeskScale } from '../composables/useDeskScale.js'
import { useI18n } from '../i18n/index.js'

// A big-screen view before its chunk is there: the page at its final layout (the hero band with the page's own icon and name, then the blocks the page is made of as
// tonal tiles with the mark and two lines a tile starts with), with a slow sweep. No delay and no wait: the async view puts it in the tree from the first frame and the
// view takes its place block for block. It has nothing to focus (no tabindex, no data-focusable), so a pad keeps its place on the sidebar while the view loads.
const props = defineProps({
  kind: { type: String, required: true },
})
const { t } = useI18n()
const scale = useDeskScale()

const PAGES = {
  dashboard: { icon: 'dashboard', title: () => 'Foundation Desktop' },
  devices: { icon: 'devices', title: (x) => x.devices.pageTitle },
  stream: { icon: 'cast_connected', title: (x) => x.stream.pageTitle },
  tools: { icon: 'build', title: (x) => x.tools.pageTitle },
  settings: { icon: 'settings', title: (x) => x.settings.pageTitle },
}
const page = computed(() => PAGES[props.kind])
const style = computed(() => (props.kind === 'dashboard' || props.kind === 'apps' ? undefined : { zoom: scale.value }))
</script>

<template>
  <div class="dsk-skel" :class="`dsk-skel--${kind}`" :style="style" role="status" aria-busy="true">
    <template v-if="kind === 'apps'">
      <div class="dsk-skel__bar"><i class="dsk-skel__pill" /><i class="dsk-skel__pill" /><i class="dsk-skel__pill" /><span class="dsk-skel__grow" /><i class="dsk-skel__dot" /><i class="dsk-skel__dot" /></div>
      <div class="dsk-skel__posters">
        <div v-for="n in 12" :key="n" class="dsk-skel__poster"><i class="dsk-skel__mark" /></div>
      </div>
    </template>

    <template v-else>
      <MHero class="dsk-skel__hero" tone="primary" :icon="page.icon" :title="page.title(t)" text="&nbsp;" />

      <template v-if="kind === 'dashboard'">
        <div class="dsk-skel__row dsk-skel__row--4">
          <div v-for="n in 4" :key="n" class="dsk-skel__tile dsk-skel__tile--stat"><i class="dsk-skel__mark" /><span class="dsk-skel__lines"><b /><b /></span></div>
        </div>
        <i class="dsk-skel__heading" />
        <div class="dsk-skel__row dsk-skel__row--4">
          <div v-for="n in 4" :key="n" class="dsk-skel__tile dsk-skel__tile--action"><i class="dsk-skel__mark" /><span class="dsk-skel__lines"><b /><b /></span></div>
        </div>
        <i class="dsk-skel__heading" />
        <div class="dsk-skel__tile dsk-skel__tile--wide"><div class="dsk-skel__row dsk-skel__row--4"><span v-for="n in 4" :key="n" class="dsk-skel__lines"><b /><b /></span></div></div>
      </template>

      <div v-else-if="kind === 'tools'" class="dsk-skel__row dsk-skel__row--2">
        <div v-for="n in 4" :key="n" class="dsk-skel__tile dsk-skel__tile--diag"><i class="dsk-skel__mark dsk-skel__mark--cookie" /><span class="dsk-skel__lines"><b /><b /></span><i class="dsk-skel__pill" /></div>
      </div>

      <div v-else-if="kind === 'stream'" class="dsk-skel__row dsk-skel__row--stream">
        <div class="dsk-skel__tile dsk-skel__tile--bit"><i class="dsk-skel__mark" /><b class="dsk-skel__number" /><i class="dsk-skel__rail" /><i class="dsk-skel__rail dsk-skel__rail--seg" /></div>
        <div class="dsk-skel__col">
          <div v-for="n in 4" :key="n" class="dsk-skel__tile dsk-skel__tile--flag"><i class="dsk-skel__mark" /><span class="dsk-skel__lines"><b /><b /></span><i class="dsk-skel__switch" /></div>
        </div>
      </div>

      <div v-else class="dsk-skel__col">
        <div v-for="n in 4" :key="n" class="dsk-skel__tile dsk-skel__tile--flag"><i class="dsk-skel__mark" /><span class="dsk-skel__lines"><b /><b /></span><i class="dsk-skel__switch" /></div>
      </div>
    </template>
  </div>
</template>

<style>
@keyframes dsk-skel-sweep { to { transform: translateX(100%); } }
.dsk-skel { box-sizing: border-box; margin: 0 auto; color: var(--md-sys-color-on-surface); }
.dsk-skel--apps { max-width: 1760px; }
.dsk-skel--dashboard { max-width: 1500px; }
.dsk-skel--devices, .dsk-skel--stream, .dsk-skel--tools, .dsk-skel--settings { max-width: 1280px; padding: 8px 0 40px; }
.dsk-skel__hero { margin-bottom: 28px; padding: 30px 40px; gap: 28px; }
.dsk-skel--dashboard .dsk-skel__hero .m-hero__shape { width: 120px; height: 120px; font-size: 56px; }
.dsk-skel--dashboard .dsk-skel__hero .m-hero__title { font: var(--m-display-medium); font-weight: 700; letter-spacing: -0.01em; color: inherit; }
.dsk-skel--dashboard .dsk-skel__hero .m-hero__text { font: var(--m-headline-small); color: inherit; }
.dsk-skel:not(.dsk-skel--dashboard) .dsk-skel__hero .m-hero__shape { width: 104px; height: 104px; font-size: 50px; }
.dsk-skel:not(.dsk-skel--dashboard) .dsk-skel__hero .m-hero__title { font: var(--m-display-small-emphasized); letter-spacing: -0.01em; }
.dsk-skel:not(.dsk-skel--dashboard) .dsk-skel__hero .m-hero__text { font: var(--m-title-large); }

.dsk-skel__row { display: grid; gap: 20px; margin-bottom: 8px; }
.dsk-skel__row--4 { grid-template-columns: repeat(4, minmax(0, 1fr)); }
.dsk-skel__row--2 { grid-template-columns: repeat(2, minmax(0, 1fr)); margin-bottom: 20px; }
.dsk-skel__row--stream { grid-template-columns: minmax(0, 1fr); }
@media (min-width: 1500px) { .dsk-skel__row--stream { grid-template-columns: minmax(0, 1.05fr) minmax(0, 1fr); align-items: stretch; } }
@media (max-width: 1300px) { .dsk-skel__row--2 { grid-template-columns: minmax(0, 1fr); } }
.dsk-skel__col { display: flex; flex-direction: column; gap: 16px; min-width: 0; }

.dsk-skel__tile, .dsk-skel__poster, .dsk-skel__bar { position: relative; overflow: hidden; display: flex; align-items: center; gap: 18px; min-width: 0; border-radius: var(--md-sys-shape-corner-extra-large, 28px); background: var(--md-sys-color-surface-container-high); }
.dsk-skel__tile--stat { padding: 22px 24px; min-height: 112px; gap: 20px; }
.dsk-skel__tile--action { padding: 22px 24px; min-height: 96px; }
.dsk-skel__tile--wide { padding: 24px; min-height: 120px; display: block; background: var(--md-sys-color-surface-container); }
.dsk-skel__tile--wide .dsk-skel__row { margin: 0; }
.dsk-skel__tile--diag { padding: 20px 22px; min-height: 112px; }
.dsk-skel__tile--flag { padding: 22px 24px; min-height: 108px; }
.dsk-skel__tile--bit { flex-direction: column; align-items: stretch; justify-content: space-between; gap: 22px; padding: 28px; min-height: 340px; }
.dsk-skel__bar { height: 64px; margin-bottom: 24px; padding: 0 24px; gap: 14px; border-radius: 9999px; background: var(--md-sys-color-surface-container); }
.dsk-skel__grow { flex: 1; }
.dsk-skel__posters { display: grid; grid-template-columns: repeat(auto-fill, minmax(230px, 1fr)); gap: 28px; }
.dsk-skel__poster { aspect-ratio: 3 / 4; align-items: flex-end; justify-content: flex-start; padding: 22px; }

/* the marks and lines read on the tile in light and dark: they are the page's own tokens, not greys */
.dsk-skel__mark { flex: none; width: 56px; height: 56px; border-radius: 50%; background: color-mix(in srgb, var(--md-sys-color-primary) 22%, transparent); }
.dsk-skel__mark--cookie { width: 72px; height: 72px; border-radius: 0; background: color-mix(in srgb, var(--md-sys-color-secondary) 30%, transparent); clip-path: var(--m-shape-cookie, circle(50%)); }
.dsk-skel__poster .dsk-skel__mark { width: 44px; height: 44px; }
.dsk-skel__lines { display: flex; flex-direction: column; gap: 10px; flex: 1; min-width: 0; }
.dsk-skel__lines > b, .dsk-skel__heading, .dsk-skel__number, .dsk-skel__pill, .dsk-skel__dot, .dsk-skel__rail, .dsk-skel__switch { display: block; background: color-mix(in srgb, var(--md-sys-color-on-surface) 14%, transparent); }
.dsk-skel__lines > b { height: 14px; width: 48%; border-radius: 7px; }
.dsk-skel__lines > b + b { width: 74%; height: 11px; }
.dsk-skel__heading { width: 220px; height: 24px; margin: 28px 0 14px; border-radius: 12px; }
.dsk-skel__number { width: 46%; height: 84px; border-radius: 24px; }
.dsk-skel__rail { height: 12px; border-radius: 6px; }
.dsk-skel__rail--seg { height: 56px; border-radius: 9999px; }
.dsk-skel__pill { flex: none; width: 96px; height: 32px; border-radius: 9999px; }
.dsk-skel__bar .dsk-skel__pill { width: 88px; height: 36px; }
.dsk-skel__dot { flex: none; width: 40px; height: 40px; border-radius: 50%; }
.dsk-skel__switch { flex: none; width: 64px; height: 36px; border-radius: 9999px; }

.dsk-skel__tile::after, .dsk-skel__poster::after, .dsk-skel__bar::after { content: ''; position: absolute; inset: 0; transform: translateX(-100%); background: linear-gradient(100deg, transparent 30%, color-mix(in srgb, var(--md-sys-color-on-surface) 9%, transparent) 50%, transparent 70%); animation: dsk-skel-sweep 2.4s ease-in-out infinite; }
@media (prefers-reduced-motion: reduce) { html:not([data-motion='full']) .dsk-skel *::after { display: none; } }
html[data-motion='reduced'] .dsk-skel *::after { display: none; }
</style>
