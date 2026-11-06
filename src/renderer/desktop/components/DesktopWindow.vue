<template>
  <div class="desktop-window m-app">
    <div class="wallpaper-layer" aria-hidden="true"></div>
    <slot name="titlebar">
      <TitleBar v-if="showTitleBar" :title="title" :icon="icon" />
    </slot>

    <div class="desktop-window-content" :class="{ 'has-sidebar': hasSidebar }">
      <slot name="sidebar"></slot>

      <main class="desktop-window-main">
        <slot></slot>
      </main>
    </div>

    <slot name="footer"></slot>
  </div>
</template>

<script setup>
import TitleBar from './TitleBar.vue'

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

.desktop-window-main {
  flex: 1;
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
