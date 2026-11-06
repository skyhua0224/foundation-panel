<template>
  <Transition name="ctx">
    <div
      v-if="visible"
      ref="menuRef"
      class="context-menu"
      role="menu"
      :style="{ left: position.left + 'px', top: position.top + 'px' }"
      @click.stop
      @keydown.esc.prevent="$emit('close')"
    >
      <MButton class="menu-item" variant="text" size="lg" icon="play_circle" data-focusable data-focus-key="ctx-launch" role="menuitem" @click="$emit('launch')">
        {{ t.appContext.launch }}
      </MButton>
      <MButton class="menu-item" variant="text" size="lg" :icon="isFavorited ? 'star' : 'favorite'" data-focusable data-focus-key="ctx-favorite" role="menuitem" @click="$emit('toggleFavorite')">
        {{ isFavorited ? t.appContext.unfavorite : t.appContext.favorite }}
      </MButton>
      <div class="menu-divider" role="separator"></div>
      <MButton class="menu-item" variant="text" size="lg" icon="image" data-focusable data-focus-key="ctx-cover" role="menuitem" @click="$emit('updateCover')">
        {{ t.appContext.updateCover }}
      </MButton>
      <MButton class="menu-item" variant="text" size="lg" icon="bolt" data-focusable data-focus-key="ctx-helpers" role="menuitem" @click="$emit('configHelpers')">
        {{ t.appContext.launchHelper }}
      </MButton>
      <MButton v-if="hasCmd" class="menu-item" variant="text" size="lg" icon="content_copy" data-focusable data-focus-key="ctx-copy" role="menuitem" @click="$emit('copyCmd')">
        {{ t.appContext.copyCommand }}
      </MButton>
      <MButton v-if="hasWorkingDir" class="menu-item" variant="text" size="lg" icon="folder" data-focusable data-focus-key="ctx-dir" role="menuitem" @click="$emit('openDir')">
        {{ t.appContext.openDirectory }}
      </MButton>
    </div>
  </Transition>
</template>

<script setup>
import { nextTick, onUnmounted, ref, watch } from 'vue'
import { useI18n } from '../i18n/index.js'
import { popFocusScope, pushFocusScope } from '../composables/useFocusNav.js'
import { MButton } from 'foundation-ui/vue'
const { t } = useI18n()

const props = defineProps({
  visible: { type: Boolean, required: true },
  x: { type: Number, default: 0 },
  y: { type: Number, default: 0 },
  isFavorited: { type: Boolean, default: false },
  hasCmd: { type: Boolean, default: false },
  hasWorkingDir: { type: Boolean, default: false },
})

defineEmits(['launch', 'toggleFavorite', 'copyCmd', 'openDir', 'configHelpers', 'updateCover', 'close'])

const menuRef = ref(null)
const position = ref({ left: 0, top: 0 })
const VIEWPORT_MARGIN = 8

let disposeScope = null

/**
 * 用菜单自己的实测尺寸夹到视口内。之前调用方按固定 200px 估算，
 * 菜单项数量变化或大屏缩放后就会溢出屏幕。
 */
function place() {
  const menu = menuRef.value
  const width = menu?.offsetWidth || 180
  const height = menu?.offsetHeight || 200
  position.value = {
    left: Math.max(VIEWPORT_MARGIN, Math.min(props.x, window.innerWidth - width - VIEWPORT_MARGIN)),
    top: Math.max(VIEWPORT_MARGIN, Math.min(props.y, window.innerHeight - height - VIEWPORT_MARGIN)),
  }
}

watch(
  () => [props.visible, props.x, props.y],
  async ([visible]) => {
    if (!visible) {
      disposeScope?.()
      disposeScope = null
      return
    }
    // 先按请求坐标渲染，再测量并纠正，避免出现一帧的错位
    position.value = { left: props.x, top: props.y }
    await nextTick()
    place()
    if (menuRef.value && !disposeScope) disposeScope = pushFocusScope(menuRef.value)
  },
  { immediate: true }
)

onUnmounted(() => {
  disposeScope?.()
})
</script>

<style lang="less" scoped>
// M3 菜单：surface-container 卡片，28 圆角，从点击处弹开
.context-menu {
  position: fixed;
  z-index: 20000;
  min-width: 360px;
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 2px;
  border-radius: 28px;
  background: var(--md-sys-color-surface-container);
  box-shadow: var(--d-shadow-3);
  transform-origin: top left;

  .menu-item {
    justify-content: flex-start;
    width: 100%;
    height: 64px;
    border-radius: 22px;
    padding: 0 24px;
    gap: 16px;
    font: 500 20px/28px var(--md-ref-typeface-brand, system-ui);
  }

  .menu-divider {
    height: 1px;
    margin: 6px 12px;
    background: var(--md-sys-color-outline-variant);
  }
}

.ctx-enter-active {
  transition: opacity var(--m-spring-fast-effects-duration, 150ms) var(--m-spring-fast-effects, ease),
    transform var(--m-spring-fast-spatial-duration, 250ms) var(--m-spring-fast-spatial, ease);
}
.ctx-leave-active {
  transition: opacity var(--m-spring-fast-effects-duration, 120ms) var(--m-spring-fast-effects, ease);
}
.ctx-enter-from,
.ctx-leave-to {
  opacity: 0;
}
.ctx-enter-from {
  transform: scale(0.9);
}
</style>
