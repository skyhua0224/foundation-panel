<template>
  <div class="apps-list">
    <div
      v-for="app in apps"
      :key="'list-' + app.name"
      class="app-list-item"
      :class="{ launching: launchingApp === app.name }"
      role="button"
      tabindex="0"
      :aria-label="app.name"
      :aria-busy="launchingApp === app.name || undefined"
      :data-app-name="app.name"
      :data-focus-key="'app-' + app.name"
      @click="$emit('launch', app)"
      @keydown.enter="$emit('launch', app)"
      @contextmenu.prevent="$emit('contextmenu', $event, app)"
    >
      <div class="list-cover">
        <img
          v-if="getAppImageUrl(app)"
          :src="getAppImageUrl(app)"
          :alt="app.name"
          loading="lazy"
          decoding="async"
          @error="handleImageError($event, app)"
        />
        <div v-else class="mini-placeholder">{{ app.name?.[0] || '?' }}</div>
      </div>
      <div class="list-info">
        <span class="list-name" data-redact="text">{{ app.name }}</span>
        <span class="list-cmd" v-if="app.cmd">{{ app.cmd }}</span>
      </div>
      <div class="list-tags">
        <MChip v-if="playtimeLabel(app.name)" icon="schedule">{{ playtimeLabel(app.name) }}</MChip>
        <MChip v-if="isFavorite(app.name)" tone="tertiary" icon="star">{{ t.appContext.favorite }}</MChip>
        <MChip v-if="app.elevated && app.elevated !== 'false'" tone="warn" icon="shield">{{ t.appContext.admin }}</MChip>
      </div>
      <span class="list-play" aria-hidden="true">
        <MLoading v-if="launchingApp === app.name" size="32px" />
        <MIcon v-else name="play_arrow" fill />
      </span>
    </div>
  </div>
</template>

<script setup>
import { useI18n } from '../i18n/index.js'
import { MChip, MIcon, MLoading } from 'foundation-ui/vue'
import { formatDuration } from '../composables/useGameSession.js'
const { t } = useI18n()

const props = defineProps({
  apps: { type: Array, required: true },
  launchingApp: { type: String, default: null },
  isFavorite: { type: Function, required: true },
  getAppImageUrl: { type: Function, required: true },
  handleImageError: { type: Function, required: true },
  stats: { type: Object, default: () => ({}) },
})

defineEmits(['launch', 'contextmenu'])

function playtimeLabel(appName) {
  const seconds = Number(props.stats?.[appName]?.totalSeconds) || 0
  if (seconds < 60) return ''
  return t.value.gameSession.playedFor.replace('{duration}', formatDuration(seconds))
}
</script>

<style lang="less" scoped>
.apps-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 6px 8px 28px;
  margin: -6px -8px 0;
}

.app-list-item {
  display: flex;
  align-items: center;
  gap: 20px;
  min-height: 96px;
  padding: 12px 24px 12px 12px;
  border-radius: 28px;
  background: var(--md-sys-color-surface-container-high);
  cursor: pointer;
  outline: none;
  transition: background-color var(--m-spring-default-effects-duration, 200ms) var(--m-spring-default-effects, ease),
    transform var(--m-spring-fast-spatial-duration, 200ms) var(--m-spring-fast-spatial, ease);

  &:hover {
    background: var(--md-sys-color-surface-container-highest);
  }

  &:active { transform: scale(0.99); }
  &.launching { pointer-events: none; opacity: 0.8; }
}

.list-cover {
  flex: none;
  width: 72px;
  height: 72px;
  border-radius: 20px;
  overflow: hidden;
  background: var(--md-sys-color-surface-container-highest);

  img { width: 100%; height: 100%; object-fit: cover; display: block; }
}

.mini-placeholder {
  width: 100%;
  height: 100%;
  display: grid;
  place-items: center;
  font: 700 30px/1 var(--md-ref-typeface-brand, system-ui);
  background: var(--md-sys-color-primary-container);
  color: var(--md-sys-color-on-primary-container);
}

.list-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;

  .list-name {
    font: 600 20px/28px var(--md-ref-typeface-brand, system-ui);
    color: var(--md-sys-color-on-surface);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .list-cmd {
    font: 400 14px/20px var(--md-ref-typeface-brand, system-ui);
    color: var(--md-sys-color-on-surface-variant);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}

.list-tags {
  display: flex;
  gap: 8px;
  flex: none;
}

.list-play {
  flex: none;
  display: grid;
  place-items: center;
  width: 56px;
  height: 56px;
  border-radius: 20px;
  background: var(--md-sys-color-primary);
  color: var(--md-sys-color-on-primary);

  .m-icon { width: 32px; height: 32px; }
}
</style>
