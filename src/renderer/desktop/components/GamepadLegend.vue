<template>
  <span class="gamepad-legend" :data-pad-type="padType" :data-cursor-mode="cursorMode ? 'on' : 'off'" aria-hidden="true">
    <TransitionGroup name="legend-item" tag="span" class="legend-items">
      <span v-for="hint in hints" :key="hint.id" class="legend-item" :class="{ 'is-primary': hint.primary }" :data-hint="hint.id">
        <PadGlyph :action="hint.action" size="md" />
        <span class="legend-label">{{ hint.label }}</span>
      </span>
    </TransitionGroup>
  </span>
</template>

<script setup>
import { computed } from 'vue'
import { viewActions } from '../composables/useGamepadActions.js'
import { bigScreenSettings } from '../composables/useBigScreenSettings.js'
import { padType } from '../composables/padType.js'
import { useI18n } from '../i18n/index.js'
import PadGlyph from './PadGlyph.vue'

const { t } = useI18n()

const props = defineProps({
  cursorMode: { type: Boolean, default: false },
})

/** 动作 id → 文案。键的图标（A/B/X/Y 还是 ✕○□△，LB 还是 L1）由 padGlyphs 按手柄类型给出，文案里不写键名。 */
const LABELS = () => {
  const legend = t.value.gamepadLegend
  return {
    click: legend.click,
    exitCursor: legend.exitCursor,
    confirm: legend.confirm,
    back: legend.back,
    search: legend.search,
    favorite: legend.favorite,
    menu: legend.menu,
    scroll: legend.scroll,
    pages: legend.switchPage,
    cursor: legend.cursorOn,
  }
}

/**
 * 只显示当前上下文真的会响应的按键：
 * 全局动作（确认/返回/换页/滚动/光标）始终在，视图级动作（搜索/收藏/菜单）
 * 由各视图通过 useGamepadActions 声明——没声明的页面就不显示，不撒谎。
 * 图标跟随当前手柄类型（padType：串流时主机报告的虚拟手柄优先，否则本机手柄的 id 与 mapping），换手柄即时切换。
 */
const hints = computed(() => {
  const labels = LABELS()
  if (props.cursorMode) {
    return [
      { id: 'click', action: 'click', label: labels.click, primary: true },
      { id: 'menu', action: 'menu', label: labels.menu },
      { id: 'exitCursor', action: 'exitCursor', label: labels.exitCursor, primary: true },
      { id: 'scroll', action: 'scroll', label: labels.scroll },
    ]
  }

  const actions = ['confirm', 'back']
  if (viewActions.value.has('search')) actions.push('search')
  if (viewActions.value.has('favorite')) actions.push('favorite')
  if (viewActions.value.has('menu')) actions.push('menu')
  actions.push('scroll', 'pages')
  // 设置里关掉光标模式时 L3 不会响应，提示条不能显示无效按键
  if (bigScreenSettings.value.gamepadCursorEnabled) actions.push('cursor')

  return actions.map((id) => ({ id, action: id, label: labels[id], primary: id === 'confirm' || id === 'back' }))
})
</script>

<style lang="less" scoped>
// 按键提示条：底部居中的大圆角药丸（与「正在运行」条同一套语言），图标 44px、字 18px，沙发十英尺能读。
// 确认与返回是主操作：字重与颜色更重，其余退一档。
.gamepad-legend {
  display: inline-flex;
  align-items: center;
  max-width: min(100%, 1800px);
  padding: 12px 30px 12px 22px;
  border-radius: 9999px;
  background: color-mix(in srgb, var(--md-sys-color-surface-container-high) 88%, transparent);
  box-shadow: var(--d-shadow-3), inset 0 0 0 1px color-mix(in srgb, var(--md-sys-color-outline-variant) 70%, transparent);
  backdrop-filter: blur(18px) saturate(1.2);
  pointer-events: none;
}

.legend-items {
  position: relative;
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  justify-content: center;
  gap: 8px 26px;
}

.legend-item {
  display: inline-flex;
  align-items: center;
  gap: 12px;
}

.legend-label {
  font: 600 18px/24px var(--md-ref-typeface-brand, system-ui);
  color: var(--md-sys-color-on-surface-variant);
  white-space: nowrap;
}
.legend-item.is-primary .legend-label {
  font-weight: 750;
  color: var(--md-sys-color-on-surface);
}

// 动作增减、手柄类型更换时：弹簧进出，位置平滑移动
.legend-item-enter-active,
.legend-item-leave-active {
  transition: opacity var(--m-spring-fast-effects-duration, 160ms) var(--m-spring-fast-effects, ease),
    transform var(--m-spring-fast-spatial-duration, 240ms) var(--m-spring-fast-spatial, ease);
}
.legend-item-move {
  transition: transform var(--m-spring-default-spatial-duration, 320ms) var(--m-spring-default-spatial, ease);
}
.legend-item-leave-active { position: absolute; }
.legend-item-enter-from,
.legend-item-leave-to {
  opacity: 0;
  transform: translateY(10px) scale(0.92);
}
@media (prefers-reduced-motion: reduce) {
  :global(html:not([data-motion='full'])) .legend-item-enter-active,
  :global(html:not([data-motion='full'])) .legend-item-leave-active,
  :global(html:not([data-motion='full'])) .legend-item-move { transition: none; }
}
:global(html[data-motion='reduced']) .legend-item-enter-active,
:global(html[data-motion='reduced']) .legend-item-leave-active,
:global(html[data-motion='reduced']) .legend-item-move { transition: none; }
</style>
