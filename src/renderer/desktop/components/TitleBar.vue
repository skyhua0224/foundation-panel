<template>
  <div class="desktop-titlebar" :class="{ 'draggable': draggable }">
    <div class="titlebar-left">
      <slot name="left">
        <img v-if="icon" :src="icon" alt="App Icon" class="app-icon" />
        <span v-if="title" class="app-title">{{ title }}</span>
      </slot>
    </div>
    
    <div class="titlebar-center">
      <slot name="center"></slot>
    </div>
    
    <div class="titlebar-right">
      <slot name="right">
        <WindowControls v-if="showControls" />
      </slot>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import WindowControls from './WindowControls.vue'

const props = defineProps({
  title: {
    type: String,
    default: 'FOUNDATION DESKTOP'
  },
  icon: {
    type: String,
    default: null
  },
  draggable: {
    type: Boolean,
    default: true
  },
  showControls: {
    type: Boolean,
    default: true
  }
})
</script>

<style lang="less" scoped>
// 小号顶栏（48）：surface-container，无分隔线，标题用 title-small
.desktop-titlebar {
  height: 48px;
  background: var(--md-sys-color-surface-container);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px 0 20px;
  position: relative;
  z-index: 100;
  user-select: none;

  &.draggable {
    -webkit-app-region: drag;
  }

  .titlebar-left {
    display: flex;
    align-items: center;
    gap: 12px;
    flex: 1;
    -webkit-app-region: no-drag;

    .app-icon {
      width: 24px;
      height: 24px;
    }

    .app-title {
      font: 500 14px/20px var(--md-ref-typeface-brand, system-ui);
      color: var(--md-sys-color-on-surface-variant);
      letter-spacing: 0.1px;
    }
  }

  .titlebar-center {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    -webkit-app-region: drag;
  }

  .titlebar-right {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    -webkit-app-region: no-drag;
  }
}
</style>
