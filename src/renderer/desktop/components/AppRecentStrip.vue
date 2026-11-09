<template>
  <section class="recent-section">
    <h2 class="section-label"><MIcon name="history" />{{ t.apps.recentSection }}</h2>
    <div class="recent-strip">
      <div
        v-for="app in apps"
        :key="'recent-' + app.name"
        class="recent-tile"
        role="button"
        tabindex="0"
        :data-app-name="app.name"
        :data-focus-key="'recent-' + app.name"
        :aria-label="app.name"
        @click="$emit('launch', app)"
        @keydown.enter="$emit('launch', app)"
        @contextmenu.prevent="$emit('contextmenu', $event, app)"
      >
        <div class="recent-cover">
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
        <span class="recent-name" data-redact="text">{{ app.name }}</span>
      </div>
    </div>
  </section>
</template>

<script setup>
import { MIcon } from 'foundation-ui/vue'
import { useI18n } from '../i18n/index.js'

const { t } = useI18n()

defineProps({
  apps: { type: Array, required: true },
  getAppImageUrl: { type: Function, required: true },
  handleImageError: { type: Function, required: true },
})

defineEmits(['launch', 'contextmenu'])
</script>

<style lang="less" scoped>
.recent-section {
  margin-bottom: 36px;
}

.section-label {
  display: flex;
  align-items: center;
  gap: 10px;
  margin: 0 0 16px;
  font: 600 20px/28px var(--md-ref-typeface-brand, system-ui);
  color: var(--md-sys-color-on-surface-variant);

  .m-icon {
    width: 26px;
    height: 26px;
    color: var(--md-sys-color-primary);
  }
}

// 横向条：焦点环和抬升不能被裁掉，所以上下留出 padding
.recent-strip {
  display: flex;
  gap: 20px;
  overflow-x: auto;
  padding: 10px 8px 18px;
  margin: -10px -8px 0;
  scrollbar-width: none;
}

.recent-tile {
  flex: none;
  width: 132px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  cursor: pointer;
  outline: none;
  border-radius: 28px;
  transition: transform var(--m-spring-fast-spatial-duration, 200ms) var(--m-spring-fast-spatial, ease);

  &:hover {
    transform: translateY(-4px);
  }

  &:active {
    transform: scale(0.97);
  }
}

.recent-cover {
  width: 132px;
  height: 132px;
  border-radius: 28px;
  overflow: hidden;
  background: var(--md-sys-color-surface-container-highest);
  box-shadow: var(--d-shadow-1);

  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
}

.mini-placeholder {
  width: 100%;
  height: 100%;
  display: grid;
  place-items: center;
  font: 700 48px/1 var(--md-ref-typeface-brand, system-ui);
  background: var(--md-sys-color-primary-container);
  color: var(--md-sys-color-on-primary-container);
}

.recent-name {
  font: 500 15px/20px var(--md-ref-typeface-brand, system-ui);
  color: var(--md-sys-color-on-surface);
  text-align: center;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
