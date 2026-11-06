<template>
  <nav class="desktop-sidebar" :class="{ collapsed }" :aria-label="label">
    <div class="rail-group">
      <div
        v-for="item in items"
        :key="item.id"
        class="nav-item"
        :class="{ active: activeItem === item.id, disabled: item.disabled }"
        role="button"
        :tabindex="item.disabled ? -1 : 0"
        :aria-current="activeItem === item.id ? 'page' : undefined"
        :aria-disabled="item.disabled || undefined"
        :data-focus-key="'nav-' + item.id"
        :title="item.label"
        @click="handleItemClick(item)"
        @keydown.enter.prevent="handleItemClick(item)"
        @keydown.space.prevent="handleItemClick(item)"
      >
        <span class="nav-pill">
          <MIcon v-if="typeof item.icon === 'string'" class="nav-icon" :name="item.icon" :fill="activeItem === item.id" />
          <component :is="item.icon" v-else-if="item.icon" class="nav-icon" />
          <MBadge v-if="item.badge && !collapsed" class="nav-badge" :value="item.badge" />
        </span>
        <span v-if="!collapsed" class="nav-label">{{ item.label }}</span>
      </div>
    </div>

    <div class="rail-group rail-foot">
      <div
        v-for="item in bottomItems"
        :key="item.id"
        class="nav-item"
        :class="{ active: activeItem === item.id, disabled: item.disabled }"
        role="button"
        :tabindex="item.disabled ? -1 : 0"
        :aria-current="activeItem === item.id ? 'page' : undefined"
        :aria-disabled="item.disabled || undefined"
        :data-focus-key="'nav-' + item.id"
        :title="item.label"
        @click="handleItemClick(item)"
        @keydown.enter.prevent="handleItemClick(item)"
        @keydown.space.prevent="handleItemClick(item)"
      >
        <span class="nav-pill">
          <MIcon v-if="typeof item.icon === 'string'" class="nav-icon" :name="item.icon" :fill="activeItem === item.id" />
          <component :is="item.icon" v-else-if="item.icon" class="nav-icon" />
        </span>
        <span v-if="!collapsed" class="nav-label">{{ item.label }}</span>
      </div>

      <MIconButton
        v-if="collapsible"
        class="rail-toggle"
        :icon="collapsed ? 'chevron_right' : 'chevron_left'"
        :label="collapsed ? expandLabel : collapseLabel"
        data-focusable
        @click="$emit('update:collapsed', !collapsed)"
      />
    </div>
  </nav>
</template>

<script setup>
import { MBadge, MIcon, MIconButton } from 'foundation-ui/vue'

// items / bottomItems: [{ id, label, icon: 'material-name' | Component, disabled?, badge? }]
defineProps({
  items: { type: Array, default: () => [] },
  bottomItems: { type: Array, default: () => [] },
  activeItem: { type: String, default: null },
  collapsed: { type: Boolean, default: false },
  collapsible: { type: Boolean, default: false },
  label: { type: String, default: 'Main' },
  expandLabel: { type: String, default: 'Expand' },
  collapseLabel: { type: String, default: 'Collapse' },
})

const emit = defineEmits(['item-click', 'update:collapsed', 'update:activeItem'])

function handleItemClick(item) {
  if (item.disabled) return
  emit('item-click', item)
  emit('update:activeItem', item.id)
}
</script>

<style lang="less" scoped>
// M3 navigation rail，放大到 10 英尺：图标 32、指示胶囊 80×48、标签 14/20
.desktop-sidebar {
  width: 120px;
  flex: none;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: space-between;
  padding: 32px 0 28px;
  background: color-mix(in srgb, var(--md-sys-color-surface-container) 92%, transparent);
  position: relative;
  z-index: 3;
  transition: width var(--m-spring-default-spatial-duration, 300ms) var(--m-spring-default-spatial, ease);

  &.collapsed {
    width: 88px;
  }
}

.rail-group {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
  width: 100%;
}

.nav-item {
  width: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  color: var(--md-sys-color-on-surface-variant);
  outline: none;
  border-radius: 24px;
  -webkit-tap-highlight-color: transparent;

  .nav-pill {
    position: relative;
    display: grid;
    place-items: center;
    width: 80px;
    height: 48px;
    border-radius: 24px;
    transition: background-color var(--m-spring-default-effects-duration, 200ms) var(--m-spring-default-effects, ease),
      border-radius var(--m-spring-fast-spatial-duration, 200ms) var(--m-spring-fast-spatial, ease);
  }

  .nav-icon {
    width: 30px;
    height: 30px;
  }

  .nav-badge {
    position: absolute;
    top: 2px;
    right: 8px;
  }

  .nav-label {
    font: 500 14px/20px var(--md-ref-typeface-brand, system-ui);
    text-align: center;
    max-width: 100%;
    padding: 0 6px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  &:hover:not(.disabled) .nav-pill {
    background: color-mix(in srgb, var(--md-sys-color-on-surface) 8%, transparent);
  }

  &:active:not(.disabled) .nav-pill {
    border-radius: 16px;
  }

  &.active {
    color: var(--md-sys-color-on-surface);

    .nav-pill {
      background: var(--md-sys-color-secondary-container);
      color: var(--md-sys-color-on-secondary-container);
    }

    .nav-label {
      font-weight: 700;
    }
  }

  &.disabled {
    opacity: 0.38;
    cursor: not-allowed;
  }
}

.rail-foot {
  gap: 10px;
}

.rail-toggle {
  margin-top: 4px;
}
</style>
