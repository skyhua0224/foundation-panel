<script setup>
import { computed } from 'vue'
import { MIcon, useNotify } from 'foundation-ui/vue'
import { useI18n } from '../i18n/index.js'
import { PANEL_LINKS, openInPanel } from '../panelLinks.js'

// "在 Panel 中打开": one tile for one page of the main panel. A big shaped icon, the page's name, what is on it, and the arrow out. Focus and
// hover morph the corners and turn the shape (foundation-ui motion tokens; 减少 stops the turning). A pad presses it like any card.
const props = defineProps({ link: { type: String, required: true } })
const { t } = useI18n()
const meta = PANEL_LINKS[props.link]
const text = computed(() => t.value.panel.links[props.link])

async function open() {
  if (!(await openInPanel(meta.page))) useNotify().snack(t.value.panel.openFailed, { timeout: 4000 })
}
</script>

<template>
  <div
    class="dsk-link"
    :class="`dsk-link--${meta.tone}`"
    role="button"
    tabindex="0"
    :data-focus-key="'panel-' + link"
    :aria-label="`${t.panel.open}: ${text.title}`"
    @click="open"
    @keydown.enter.prevent="open"
    @keydown.space.prevent="open"
  >
    <span class="dsk-link__art" aria-hidden="true"><span class="dsk-link__shape" /><MIcon :name="meta.icon" fill /></span>
    <span class="dsk-link__body">
      <span class="dsk-link__title">{{ text.title }}</span>
      <span class="dsk-link__items">{{ text.items }}</span>
    </span>
    <span class="dsk-link__go" aria-hidden="true"><MIcon name="open_in_new" /></span>
  </div>
</template>

<style scoped>
.dsk-link { --l-bg: var(--md-sys-color-primary-container); --l-fg: var(--md-sys-color-on-primary-container); --l-accent: var(--md-sys-color-primary); --l-on: var(--md-sys-color-on-primary);
  position: relative; overflow: hidden; display: flex; align-items: center; gap: 20px; min-width: 0; min-height: 112px; padding: 18px 22px 18px 18px; cursor: pointer; outline: none;
  border-radius: var(--md-sys-shape-corner-extra-large); background: var(--l-bg); color: var(--l-fg);
  transition: border-radius var(--m-spring-default-spatial-duration, 300ms) var(--m-spring-default-spatial, ease), transform var(--m-spring-fast-spatial-duration, 200ms) var(--m-spring-fast-spatial, ease); }
.dsk-link--tertiary { --l-bg: var(--md-sys-color-tertiary-container); --l-fg: var(--md-sys-color-on-tertiary-container); --l-accent: var(--md-sys-color-tertiary); --l-on: var(--md-sys-color-on-tertiary); }
.dsk-link--secondary { --l-bg: var(--md-sys-color-secondary-container); --l-fg: var(--md-sys-color-on-secondary-container); --l-accent: var(--md-sys-color-secondary); --l-on: var(--md-sys-color-on-secondary); }
.dsk-link--neutral { --l-bg: var(--md-sys-color-surface-container-high); --l-fg: var(--md-sys-color-on-surface); --l-accent: var(--md-sys-color-outline); --l-on: var(--md-sys-color-surface); }
.dsk-link:hover, .dsk-link:focus-visible, .dsk-link:focus { border-radius: var(--md-sys-shape-corner-extra-extra-large); transform: translateY(-3px); }
.dsk-link:active { transform: scale(.98); }
.dsk-link__art { position: relative; flex: none; width: 76px; height: 76px; display: grid; place-items: center; font-size: 36px; color: var(--l-on); }
.dsk-link__shape { position: absolute; inset: 0; background: var(--l-accent); clip-path: var(--m-shape-cookie); animation: m-turn 40s linear infinite; transition: transform var(--m-spring-default-spatial-duration, 300ms) var(--m-spring-default-spatial, ease); }
.dsk-link:hover .dsk-link__shape, .dsk-link:focus .dsk-link__shape { transform: rotate(45deg) scale(1.08); }
.dsk-link__art :deep(.m-icon) { position: relative; }
.dsk-link__body { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }
.dsk-link__title { font: var(--m-title-large); overflow-wrap: anywhere; }
.dsk-link__items { font: var(--m-body-medium); opacity: .82; overflow-wrap: anywhere; }
.dsk-link__go { flex: none; width: 52px; height: 52px; display: grid; place-items: center; border-radius: 50%; background: color-mix(in srgb, var(--l-fg) 12%, transparent); font-size: 26px; }
</style>
