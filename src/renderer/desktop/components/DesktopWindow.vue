<template>
  <div ref="root" class="desktop-window m-app">
    <div class="wallpaper-layer" aria-hidden="true"></div>
    <slot name="titlebar">
      <TitleBar v-if="showTitleBar" :title="title" :icon="icon" />
    </slot>

    <div class="desktop-window-content" :class="{ 'has-sidebar': hasSidebar }">
      <slot name="sidebar"></slot>

      <div class="desktop-window-column">
        <main class="desktop-window-main">
          <slot></slot>
        </main>
        <!-- the action bar of a page (DeskDock) lives here: a row of its own under the scrolling area, clear of the gamepad legend -->
        <div id="dsk-dock" class="desktop-window-dock"></div>
      </div>
    </div>

    <slot name="footer"></slot>
  </div>
</template>

<script setup>
import { onBeforeUnmount, onMounted, ref } from 'vue'
import TitleBar from './TitleBar.vue'

// The footer (running game bar, gamepad legend) floats over the bottom edge; the dock keeps clear of it by its measured height.
const root = ref(null)
let observer = null
onMounted(() => {
  const foot = root.value?.querySelector('.desktop-footer-dock')
  if (!foot || typeof ResizeObserver === 'undefined') return
  const apply = () => root.value?.style.setProperty('--dsk-foot-h', `${foot.offsetHeight + 28}px`)
  observer = new ResizeObserver(apply)
  observer.observe(foot)
  apply()
})
onBeforeUnmount(() => observer?.disconnect())

defineProps({
  title: { type: String, default: 'Desktop Application' },
  icon: { type: String, default: null },
  showTitleBar: { type: Boolean, default: true },
  hasSidebar: { type: Boolean, default: false },
})
</script>

<style lang="less" scoped>
// 窗口：surface 底，壁纸（主题编辑器设置的 --fd-wallpaper）压一层 scrim 让字永远读得出
.desktop-window {
  width: 100vw;
  height: 100vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  position: relative;
  min-height: 0;
  background: var(--md-sys-color-surface);
  color: var(--md-sys-color-on-surface);
}

.wallpaper-layer {
  position: absolute;
  inset: 0;
  z-index: 0;
  background-image: var(--fd-wallpaper, none);
  background-size: cover;
  background-position: center;
  background-repeat: no-repeat;
  transition: opacity var(--m-spring-slow-effects-duration, 500ms) var(--m-spring-slow-effects, ease);

  &::after {
    content: '';
    position: absolute;
    inset: 0;
    background: color-mix(in srgb, var(--md-sys-color-surface) 72%, transparent);
  }
}

.desktop-window-content {
  flex: 1;
  display: flex;
  position: relative;
  z-index: 2;
  min-height: 0;
  overflow: hidden;
}

.desktop-window-column {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.desktop-window-dock {
  flex: none;
  position: relative;
  z-index: 3;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 8px 56px var(--dsk-foot-h, 28px);

  &:empty {
    display: none;
  }
}

.desktop-window-main {
  flex: 1;
  min-height: 0;
  min-width: 0;
  padding: 40px 56px 120px;
  overflow-y: auto;
  overflow-x: hidden;
  position: relative;
  z-index: 2;

  @media (max-width: 900px) {
    padding: 28px 28px 120px;
  }
}
</style>
