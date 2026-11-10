<template>
  <div class="apps-grid" :class="'grid-' + gridSize">
    <div
      v-for="(app, index) in apps"
      :key="app.name"
      class="app-tile fade-in"
      :class="{ launching: launchingApp === app.name, favorited: isFavorite(app.name) }"
      role="button"
      tabindex="0"
      :aria-label="app.name"
      :aria-busy="launchingApp === app.name || undefined"
      :data-app-name="app.name"
      :data-focus-key="'app-' + app.name"
      :style="{ animationDelay: `${Math.min(index * 0.03, 0.4)}s` }"
      @click="$emit('launch', app)"
      @keydown.enter="$emit('launch', app)"
      @contextmenu.prevent="$emit('contextmenu', $event, app)"
    >
      <div class="tile-cover">
        <img
          v-if="getAppImageUrl(app)"
          :src="getAppImageUrl(app)"
          :alt="app.name"
          class="cover-image"
          loading="lazy"
          decoding="async"
          @error="handleImageError($event, app)"
        />
        <div v-else class="cover-placeholder">
          <span class="placeholder-letter">{{ app.name?.[0] || '?' }}</span>
        </div>

        <span
          v-if="isFavorite(app.name)"
          class="favorite-badge"
          :title="t.appContext.unfavorite"
          @click.stop="$emit('toggleFavorite', app.name)"
        ><MIcon name="star" fill /></span>

        <div v-if="helperIds(app.name).length" class="helper-badges">
          <LaunchHelperIcon
            v-for="helperId in helperIds(app.name)"
            :key="helperId"
            :template-id="helperId"
            :size="16"
            class="helper-badge-icon"
          />
        </div>

        <div class="tile-overlay" aria-hidden="true">
          <span class="play-icon"><MIcon name="play_arrow" fill /></span>
        </div>

        <div v-if="launchingApp === app.name" class="launch-overlay">
          <MLoading size="56px" />
          <span>{{ t.appContext.launching }}</span>
        </div>
      </div>

      <div class="tile-info">
        <span class="tile-name" data-redact="text" :title="app.name">{{ app.name }}</span>
        <span v-if="playtimeLabel(app.name)" class="tile-playtime">{{ playtimeLabel(app.name) }}</span>
        <MChip v-if="app.elevated && app.elevated !== 'false'" class="tile-badge" tone="warn" icon="shield">{{ t.appContext.admin }}</MChip>
        <MChipRow v-if="spxTags(app).length" class="tile-chips" :chips="spxTags(app)" :more-label="sparxieMore(locale)" />
      </div>
    </div>
  </div>
</template>

<script setup>
import { useI18n } from '../i18n/index.js'
import { onMounted } from 'vue'
import { MChip, MChipRow, MIcon, MLoading } from 'foundation-ui/vue'
import { sparxieMore, sparxieTags } from '../composables/sparxieTags.js'
import { refreshSparxiePresence, useSparxiePresence } from '../composables/useSparxiePresence.js'
import LaunchHelperIcon from './LaunchHelperIcon.vue'
import { formatDuration } from '../composables/useGameSession.js'
const { t, locale } = useI18n()
// a Sparxie application shows its marks under the name while the host reports the component installed
const spx = useSparxiePresence()
onMounted(refreshSparxiePresence)
// the Sparxie mark is a picture of the panel's public assets (the art of 暗檬's character; see public/sparxie/SOURCE.md)
const SPARXIE_MARK = '/sparxie/sparxie-mark-64.png'
const spxTags = (app) => sparxieTags(app, locale.value, spx.installed).map((tag) => (tag.mark ? { ...tag, image: SPARXIE_MARK } : tag))

const props = defineProps({
  apps: { type: Array, required: true },
  gridSize: { type: String, required: true },
  launchingApp: { type: String, default: null },
  isFavorite: { type: Function, required: true },
  getAppImageUrl: { type: Function, required: true },
  handleImageError: { type: Function, required: true },
  helperIds: { type: Function, default: () => [] },
  stats: { type: Object, default: () => ({}) },
})

defineEmits(['launch', 'contextmenu', 'toggleFavorite'])

/** 累计时长不足一分钟就不显示，避免失败的启动留下 "0m" 噪声。 */
function playtimeLabel(appName) {
  const seconds = Number(props.stats?.[appName]?.totalSeconds) || 0
  if (seconds < 60) return ''
  return t.value.gameSession.playedFor.replace('{duration}', formatDuration(seconds))
}
</script>

<style lang="less" scoped>
.apps-grid {
  display: grid;
  gap: 28px;
  padding: 12px 8px 28px;
  margin: -12px -8px 0;

  &.grid-small { grid-template-columns: repeat(auto-fill, minmax(170px, 1fr)); }
  &.grid-medium { grid-template-columns: repeat(auto-fill, minmax(230px, 1fr)); }
  &.grid-large { grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); }
}

// 一张卡：surface-container-high，封面撑满上部（28 圆角），名字在下
.app-tile {
  display: flex;
  flex-direction: column;
  border-radius: var(--d-radius-tile);
  background: var(--md-sys-color-surface-container-high);
  cursor: pointer;
  outline: none;
  overflow: hidden;
  box-shadow: var(--d-shadow-1);
  transition: transform var(--m-spring-fast-spatial-duration, 200ms) var(--m-spring-fast-spatial, ease),
    box-shadow var(--m-spring-default-effects-duration, 200ms) var(--m-spring-default-effects, ease);

  &:hover {
    transform: translateY(-4px);
    box-shadow: var(--d-shadow-3);

    .tile-overlay { opacity: 1; }
  }

  &:active { transform: scale(0.98); }
}

.tile-cover {
  position: relative;
  aspect-ratio: 3 / 4;
  overflow: hidden;
  background: var(--md-sys-color-surface-container-highest);

  .cover-image {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .cover-placeholder {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    background: var(--md-sys-color-primary-container);
    color: var(--md-sys-color-on-primary-container);

    .placeholder-letter {
      font: 800 84px/1 var(--md-ref-typeface-brand, system-ui);
    }
  }
}

// 角标只给鼠标用：手柄用 X 切换收藏，所以它不是可聚焦控件
.favorite-badge {
  position: absolute;
  top: 12px;
  right: 12px;
  display: grid;
  place-items: center;
  width: 44px;
  height: 44px;
  border-radius: 16px;
  background: var(--dsk-tert-bg);
  color: var(--dsk-tert-fg);

  .m-icon { width: 26px; height: 26px; }
}

.helper-badges {
  position: absolute;
  left: 12px;
  bottom: 12px;
  display: flex;
  gap: 6px;

  .helper-badge-icon {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: 12px;
    background: var(--md-sys-color-secondary-container);
    color: var(--md-sys-color-on-secondary-container);
  }
}

.tile-overlay {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  opacity: 0;
  background: color-mix(in srgb, var(--md-sys-color-scrim, #000) 36%, transparent);
  transition: opacity var(--m-spring-default-effects-duration, 200ms) var(--m-spring-default-effects, ease);

  .play-icon {
    display: grid;
    place-items: center;
    width: 88px;
    height: 88px;
    border-radius: 32px;
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);

    .m-icon { width: 48px; height: 48px; }
  }
}

.launch-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 14px;
  background: color-mix(in srgb, var(--md-sys-color-scrim, #000) 56%, transparent);
  color: #fff;
  font: 600 16px/24px var(--md-ref-typeface-brand, system-ui);
}

.tile-info {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 4px 10px;
  padding: 16px 20px 18px;
  min-height: 78px;

  .tile-name {
    flex: 1 1 100%;
    font: 600 18px/26px var(--md-ref-typeface-brand, system-ui);
    color: var(--md-sys-color-on-surface);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tile-playtime {
    font: 400 14px/20px var(--md-ref-typeface-brand, system-ui);
    color: var(--md-sys-color-on-surface-variant);
  }

  // the marks of a Sparxie application: one row (MChipRow turns what does not fit into +N), so the tiles keep one height
  .tile-chips { flex: 1 1 100%; height: 28px; }
}

.app-tile.launching { pointer-events: none; }
</style>
