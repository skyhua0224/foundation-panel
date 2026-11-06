<template>
  <header class="apps-toolbar">
    <span class="apps-hero" aria-hidden="true"><MIcon name="apps" fill /></span>
    <h1 class="apps-title">{{ t.apps.libraryTitle }}</h1>

    <div class="filter-tabs" role="group" :aria-label="t.apps.libraryTitle">
      <MButton
        v-for="tab in filterTabs"
        :key="tab.id"
        size="lg"
        :variant="activeFilter === tab.id ? 'tonal' : 'text'"
        :aria-pressed="activeFilter === tab.id"
        :icon="activeFilter === tab.id ? 'check' : ''"
        data-focusable
        :data-focus-key="'filter-' + tab.id"
        @click="$emit('update:activeFilter', tab.id)"
      >
        {{ tab.label }}
        <span v-if="tab.count !== undefined" class="tab-count">{{ tab.count }}</span>
      </MButton>
    </div>

    <div class="toolbar-spacer"></div>

    <MField
      class="apps-search"
      type="search"
      :label="t.apps.searchPlaceholder"
      :model-value="searchQuery"
      data-focusable
      data-focus-key="apps-search"
      :aria-label="t.apps.searchPlaceholder"
      @update:model-value="$emit('update:searchQuery', $event)"
    />

    <div class="toolbar-tools">
      <MButton
        variant="outlined"
        size="lg"
        icon="swap_vert"
        data-focusable
        data-focus-key="apps-sort"
        :title="t.apps.sortTitle"
        @click="$emit('cycleSortMode')"
      >{{ sortLabel }}</MButton>

      <MButton
        v-if="viewMode === 'grid'"
        variant="outlined"
        size="lg"
        icon="grid_view"
        data-focusable
        data-focus-key="apps-size"
        :title="t.apps.cardSizeTitle"
        @click="$emit('cycleGridSize')"
      >{{ t.apps.gridSizes?.[gridSize] || gridSize }}</MButton>

      <MSeg
        size="lg"
        icon-only
        :label="t.apps.gridView + ' / ' + t.apps.listView"
        :options="viewOptions"
        :model-value="viewMode"
        @update:model-value="onViewChange"
      />
    </div>

    <MChip class="apps-count">{{ t.apps.count.replace('{count}', totalCount) }}</MChip>
  </header>
</template>

<script setup>
import { computed } from 'vue'
import { MButton, MChip, MField, MIcon, MSeg } from 'foundation-ui/vue'
import { useI18n } from '../i18n/index.js'

const { t } = useI18n()

const props = defineProps({
  filterTabs: { type: Array, required: true },
  activeFilter: { type: String, required: true },
  searchQuery: { type: String, required: true },
  sortLabel: { type: String, required: true },
  gridSize: { type: String, required: true },
  viewMode: { type: String, required: true },
  totalCount: { type: Number, required: true },
})

const emit = defineEmits([
  'update:activeFilter',
  'update:searchQuery',
  'cycleSortMode',
  'cycleGridSize',
  'toggleViewMode',
])

const viewOptions = computed(() => [
  { value: 'grid', label: t.value.apps.gridView, icon: 'grid_view' },
  { value: 'list', label: t.value.apps.listView, icon: 'list' },
])

// 视图模式的真值在 useApps（toggle 事件）；只在选了另一边时才翻转
function onViewChange(next) {
  if (next && next !== props.viewMode) emit('toggleViewMode')
}
</script>

<style lang="less" scoped>
// the library's hero band: the same language as the other pages (a burst shape with the icon, the name big), and the controls inside it
.apps-toolbar {
  position: relative;
  overflow: hidden;
  isolation: isolate;
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 16px 20px;
  margin-bottom: 28px;
  padding: 20px 28px;
  border-radius: var(--md-sys-shape-corner-extra-extra-large);
  background: var(--md-sys-color-surface-container-high);

  &::after {
    content: '';
    position: absolute;
    z-index: -1;
    right: -60px;
    top: -80px;
    width: 260px;
    height: 260px;
    background: var(--md-sys-color-primary);
    opacity: 0.08;
    clip-path: var(--m-shape-cookie);
    animation: m-turn 48s linear infinite;
  }
}

.apps-hero {
  position: relative;
  flex: none;
  display: grid;
  place-items: center;
  width: 80px;
  height: 80px;
  font-size: 38px;
  color: var(--md-sys-color-on-primary);

  &::before {
    content: '';
    position: absolute;
    inset: 0;
    background: var(--md-sys-color-primary);
    clip-path: var(--m-shape-burst);
    animation: m-turn 40s linear infinite;
  }

  .m-icon { position: relative; }
}

.apps-title {
  margin: 0 12px 0 0;
  font: var(--m-display-small-emphasized);
  letter-spacing: -0.5px;
  color: var(--md-sys-color-on-surface);
}

.filter-tabs {
  display: flex;
  gap: 8px;
}

.tab-count {
  margin-left: 8px;
  padding: 0 10px;
  min-width: 28px;
  border-radius: 9999px;
  background: color-mix(in srgb, currentColor 14%, transparent);
  font: 600 14px/24px var(--md-ref-typeface-brand, system-ui);
  text-align: center;
}

.toolbar-spacer {
  flex: 1;
}

.apps-search {
  width: 340px;
  flex: none;
}

.toolbar-tools {
  display: flex;
  align-items: center;
  gap: 12px;
}

.apps-count {
  font-size: 15px;
}
</style>
